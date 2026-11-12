// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Experimental Unix signer backed by the pinned, patched C implementation.
//! This is a research interface. The native ideal-to-isogeny implementation
//! has not been audited for secret-dependent timing or memory access.
#![forbid(unsafe_code)]

use prism_field::{ChallengeIndex, Level, Level1, Level3, Level5};
use prism_verify::{
    signature::canonical_sign, verify, ChallengeScalar, KeyIdentity, Nist, Response,
};
use sha3::{
    digest::{core_api::CoreWrapper, ExtendableOutput, Update, XofReader},
    CShake256, CShake256Core,
};
use std::os::unix::fs::DirBuilderExt;
use std::{
    fs::{self, DirBuilder, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
use zeroize::Zeroizing;

/// A secret seed. Deliberately has no Debug or serialization implementation.
pub struct SecretSeed(Zeroizing<[u8; 48]>);
impl SecretSeed {
    pub fn from_bytes(bytes: [u8; 48]) -> Self {
        Self(Zeroizing::new(bytes))
    }
    pub fn generate() -> io::Result<Self> {
        let mut bytes = Zeroizing::new([0u8; 48]);
        File::open("/dev/urandom")?.read_exact(&mut *bytes)?;
        Ok(Self(bytes))
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Backend,
    Malformed,
    Exhausted,
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Io(_) => "signer I/O failed",
            Self::Backend => "native signing backend failed",
            Self::Malformed => "native backend returned malformed public data",
            Self::Exhausted => "deterministic signing search exhausted its bound",
        })
    }
}
impl std::error::Error for Error {}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> io::Result<Self> {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        for _ in 0..64 {
            let path = std::env::temp_dir().join(format!(
                "prism-sign-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            match DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "no private scratch directory",
        ))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The level-specific `prism_signer_lvl{1,3,5}` executable built with
/// `scripts/export_s5_vectors.sh backend`. Never use the fixture exporter.
pub struct Signer {
    backend: PathBuf,
    level: Nist,
    seed: SecretSeed,
}
impl Signer {
    pub fn new(backend: impl AsRef<Path>, level: Nist, seed: SecretSeed) -> Self {
        Self {
            backend: backend.as_ref().to_path_buf(),
            level,
            seed,
        }
    }
    pub fn public_key(&self) -> Result<Vec<u8>, Error> {
        match self.level {
            Nist::I => self.public::<Level1>(),
            Nist::III => self.public::<Level3>(),
            Nist::V => self.public::<Level5>(),
        }
    }
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        match self.level {
            Nist::I => self.sign_level::<Level1>(message),
            Nist::III => self.sign_level::<Level3>(message),
            Nist::V => self.sign_level::<Level5>(message),
        }
    }
    fn stream_seed(&self, label: &[u8], context: &[u8]) -> Zeroizing<[u8; 48]> {
        let mut hash: CShake256 =
            CoreWrapper::from_core(CShake256Core::new(b"shekyl-prism-secret-v1"));
        hash.update(&self.seed.0[..]);
        hash.update(self.level.name().as_bytes());
        hash.update(&(label.len() as u64).to_le_bytes());
        hash.update(label);
        hash.update(context);
        let mut seed = Zeroizing::new([0u8; 48]);
        XofReader::read(&mut hash.finalize_xof(), &mut *seed);
        seed
    }
    fn run(&self, args: &[&std::ffi::OsStr], seed: &[u8; 48]) -> Result<String, Error> {
        let mut child = Command::new(&self.backend)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child.stdin.take().ok_or(Error::Backend)?.write_all(seed);
        if input.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Backend);
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(Error::Backend);
        }
        String::from_utf8(output.stdout).map_err(|_| Error::Malformed)
    }
    fn key<L: Level>(&self, scratch: &Scratch) -> Result<(Vec<u8>, PathBuf), Error> {
        // Native failures are surfaced rather than replaced by randomness.
        let version = Command::new(&self.backend)
            .arg("backend-version")
            .output()?;
        if !version.status.success()
            || version.stdout != format!("prism-signer-shake256-v1 {}\n", L::E).as_bytes()
        {
            return Err(Error::Backend);
        }
        let path = scratch.0.join("secret");
        let seed = self.stream_seed(b"keygen", b"");
        let text = self.run(&["keygen".as_ref(), path.as_os_str()], &seed)?;
        let pk = decode_hex(line(&text, "A ")?)?;
        KeyIdentity::<L>::decode(&pk).map_err(|_| Error::Malformed)?;
        Ok((pk, path))
    }
    fn public<L: Level>(&self) -> Result<Vec<u8>, Error> {
        let scratch = Scratch::new()?;
        Ok(self.key::<L>(&scratch)?.0)
    }
    fn sign_level<L: Level>(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        let scratch = Scratch::new()?;
        let (pk, path) = self.key::<L>(&scratch)?;
        let id = KeyIdentity::<L>::decode(&pk).map_err(|_| Error::Malformed)?;
        let mut primes = 0;
        for index in ChallengeIndex::search_order() {
            let scalar = ChallengeScalar::derive(&id, message, index);
            if !scalar.degree_is_prime::<L>() {
                continue;
            }
            primes += 1;
            // A bounded research API: never hide a failed stream behind an
            // unbounded retry. Repeated calls use exactly the same streams.
            if primes > 16 {
                return Err(Error::Exhausted);
            }
            // The signature is deterministic for a degree, even if two
            // messages or indices happen to derive that same degree.
            let seed = self.stream_seed(b"sign", scalar.as_bytes());
            let degree_scalar = decimal(scalar.as_bytes());
            let text = self.run(
                &["sign".as_ref(), path.as_os_str(), degree_scalar.as_ref()],
                &seed,
            )?;
            let mut signature = Vec::new();
            for name in ["E ", "Px ", "Py ", "Qx ", "Qy "] {
                signature.extend(decode_hex(line(&text, name)?)?);
            }
            signature.extend(index.to_canonical_bytes());
            let mut response = Response::<L>::decode(&signature).map_err(|_| Error::Malformed)?;
            if !canonical_sign::<L>(response.p.y) {
                use prism_field::Field;
                response.p.y = response.p.y.negate();
                response.q.y = response.q.y.negate();
            }
            let signature = response.encode();
            // Never release a response unless the complete Rust verifier
            // accepts its exact bytes for this public key and message.
            if verify::<L>(&pk, message, &signature).is_ok() {
                return Ok(signature);
            }
        }
        Err(Error::Exhausted)
    }
}
fn line<'a>(text: &'a str, prefix: &str) -> Result<&'a str, Error> {
    let mut rows = text.lines().filter_map(|s| s.strip_prefix(prefix));
    let value = rows.next().ok_or(Error::Malformed)?;
    if rows.next().is_some() {
        return Err(Error::Malformed);
    }
    Ok(value)
}
fn decode_hex(text: &str) -> Result<Vec<u8>, Error> {
    if !text.len().is_multiple_of(2) {
        return Err(Error::Malformed);
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|s| {
            u8::from_str_radix(std::str::from_utf8(s).map_err(|_| Error::Malformed)?, 16)
                .map_err(|_| Error::Malformed)
        })
        .collect()
}
fn decimal(bytes: &[u8]) -> String {
    let mut digits = vec![0u16];
    for byte in bytes.iter().rev() {
        let mut carry = u16::from(*byte);
        for d in &mut digits {
            let n = *d * 256 + carry;
            *d = n % 10;
            carry = n / 10;
        }
        while carry != 0 {
            digits.push(carry % 10);
            carry /= 10;
        }
    }
    digits
        .iter()
        .rev()
        .map(|d| char::from(b'0' + *d as u8))
        .collect()
}
