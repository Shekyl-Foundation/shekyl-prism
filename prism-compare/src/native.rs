// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Research-only direct adapter to the real pinned SHAKE native backend.
//! No mock isogenies: every released research response runs the real relation.
use prism_field::{Field, Level, Salt};
use prism_verify::{ChallengeScalar, KeyIdentity, Response, VerifyError};
use std::os::unix::fs::DirBuilderExt;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
use zeroize::Zeroizing;

pub struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self, &'static str> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..64 {
            let p = std::env::temp_dir().join(format!(
                "prism-compare-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::DirBuilder::new().mode(0o700).create(&p) {
                Ok(()) => return Ok(Self(p)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err("private research scratch failed"),
            }
        }
        Err("private research scratch exhausted")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[derive(Debug)]
pub enum CandidateError {
    Backend,
    Malformed,
}
#[derive(Default)]
pub struct Counts {
    pub hashes: u64,
    pub primes: u32,
    pub backend_errors: u32,
    pub malformed: u32,
    pub rejections: u32,
}
pub struct Signed {
    pub signature: Vec<u8>,
    pub scalar: Vec<u8>,
    pub counts: Counts,
}

pub struct Native<L: Level> {
    backend: PathBuf,
    scratch: Scratch,
    seed: Zeroizing<[u8; 48]>,
    pub key: Vec<u8>,
    level: core::marker::PhantomData<L>,
}
impl<L: Level> Native<L> {
    pub fn new(backend: impl AsRef<Path>, seed: Zeroizing<[u8; 48]>) -> Result<Self, &'static str> {
        let backend = backend.as_ref().to_path_buf();
        let v = Command::new(&backend)
            .arg("backend-version")
            .output()
            .map_err(|_| "backend version failed")?;
        if !v.status.success()
            || v.stdout != format!("prism-signer-shake256-v1 {}\n", L::E).as_bytes()
        {
            return Err("not the SHAKE signer backend");
        }
        let mut native = Self {
            backend,
            scratch: Scratch::new()?,
            seed,
            key: Vec::new(),
            level: core::marker::PhantomData,
        };
        let seed = super::secret_stream::<L>(&native.seed, b"keygen", b"");
        let text = native
            .run(
                &[
                    "keygen".as_ref(),
                    native.scratch.0.join("secret").as_os_str(),
                ],
                &seed,
            )
            .map_err(|_| "real native keygen failed")?;
        native.key =
            super::decode(line(&text, "A ").map_err(|_| "keygen output")?).ok_or("keygen hex")?;
        KeyIdentity::<L>::decode(&native.key).map_err(|_| "keygen identity refused")?;
        Ok(native)
    }
    fn run(&self, args: &[&std::ffi::OsStr], seed: &[u8; 48]) -> Result<String, CandidateError> {
        let mut child = Command::new(&self.backend)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| CandidateError::Backend)?;
        let input = child
            .stdin
            .take()
            .ok_or(CandidateError::Backend)?
            .write_all(seed);
        if input.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CandidateError::Backend);
        }
        let out = child
            .wait_with_output()
            .map_err(|_| CandidateError::Backend)?;
        if !out.status.success() {
            return Err(CandidateError::Backend);
        }
        String::from_utf8(out.stdout).map_err(|_| CandidateError::Malformed)
    }
    /// An unchecked native candidate; callers must run verify_salted or the
    /// archived adapter before counting a real signature or forgery.
    pub fn candidate(&self, scalar: &[u8], salt: Salt<L>) -> Result<Vec<u8>, CandidateError> {
        // Only the challenge shrinks: the per-degree RNG input keeps the
        // production row's canonical little-endian scalar byte length.
        if scalar.len() > L::S_LEN {
            return Err(CandidateError::Malformed);
        }
        let mut canonical = scalar.to_vec();
        canonical.resize(L::S_LEN, 0);
        let seed = super::secret_stream::<L>(&self.seed, b"sign", &canonical);
        let scalar = super::decimal(&canonical);
        let text = self.run(
            &[
                "sign".as_ref(),
                self.scratch.0.join("secret").as_os_str(),
                scalar.as_ref(),
            ],
            &seed,
        )?;
        let mut bytes = Vec::new();
        for name in ["E ", "Px ", "Py ", "Qx ", "Qy "] {
            bytes.extend(super::decode(line(&text, name)?).ok_or(CandidateError::Malformed)?);
        }
        bytes.extend(salt.as_bytes());
        let mut response = Response::<L>::decode(&bytes).map_err(|_| CandidateError::Malformed)?;
        if !prism_verify::signature::canonical_sign::<L>(response.p.y) {
            response.p.y = response.p.y.negate();
            response.q.y = response.q.y.negate();
        }
        Ok(response.encode())
    }
    pub fn sign(
        &self,
        message: &[u8],
        randomness: &[u8; 32],
        bits: u32,
    ) -> Result<Signed, &'static str> {
        let id = KeyIdentity::<L>::decode(&self.key).map_err(|_| "key identity")?;
        let mut counts = Counts::default();
        for counter in 0..65536 {
            if counts.primes == 16 {
                return Err("research signing exhausted 16 prime candidates");
            }
            counts.hashes += 1;
            let salt =
                super::derived_salt::<L>(&self.seed, &self.key, message, randomness, counter);
            let s =
                ChallengeScalar::derive_width(&id, message, salt, bits).ok_or("research width")?;
            if !s.degree_is_prime::<L>() {
                continue;
            }
            counts.primes += 1;
            let signature = match self.candidate(s.as_bytes(), salt) {
                Ok(s) => s,
                Err(CandidateError::Backend) => {
                    counts.backend_errors += 1;
                    continue;
                }
                Err(CandidateError::Malformed) => {
                    counts.malformed += 1;
                    continue;
                }
            };
            if verify_salted::<L>(&self.key, message, &signature, bits).is_err() {
                counts.rejections += 1;
                continue;
            }
            return Ok(Signed {
                signature,
                scalar: s.as_bytes().to_vec(),
                counts,
            });
        }
        Err("research signing exhausted 65536 counters")
    }
}
/// What the archived index signer did with one message.
#[derive(Debug, PartialEq, Eq)]
pub enum Unsalted {
    /// `signature` is the archived wire: five elements and the LE16 index.
    /// `skipped` counts earlier prime indices whose response was refused.
    Signed {
        signature: Vec<u8>,
        index: u16,
        skipped: u32,
    },
    /// The native backend failed at this prime index. The archived signer
    /// returned the error; the same streams fail again on every retry.
    Backend {
        index: u16,
    },
    Malformed {
        index: u16,
    },
    /// More than 16 prime indices, or no accepted index at all.
    Exhausted,
}
impl<L: Level> Native<L> {
    /// The archived signing loop of `archive/unsalted-index-2026-11-12`
    /// (`prism-signer` `sign_level`), on the same backend and the same
    /// scalar-keyed stream: ascending index, first prime degree, a backend
    /// or malformed result ends the call, a refused response moves on.
    pub fn sign_unsalted(&self, message: &[u8], bits: u32) -> Option<Unsalted> {
        let zero = Salt::<L>::from_bytes(&vec![0; L::SALT_LEN])?;
        let prefix = L::SIGNATURE_ELEMENTS * L::FP2_LEN;
        let mut primes = 0;
        for index in 0..=u16::MAX {
            let scalar = super::unsalted::<L>(&self.key, message, index, bits)?;
            if !prism_verify::degree_is_prime(L::E - 2, &scalar) {
                continue;
            }
            primes += 1;
            if primes > 16 {
                return Some(Unsalted::Exhausted);
            }
            let mut signature = match self.candidate(&scalar, zero) {
                Ok(s) => s,
                Err(CandidateError::Backend) => return Some(Unsalted::Backend { index }),
                Err(CandidateError::Malformed) => return Some(Unsalted::Malformed { index }),
            };
            signature.truncate(prefix);
            signature.extend(index.to_le_bytes());
            if super::verify_unsalted::<L>(&self.key, message, &signature, bits).is_ok() {
                return Some(Unsalted::Signed {
                    signature,
                    index,
                    skipped: primes - 1,
                });
            }
        }
        Some(Unsalted::Exhausted)
    }
}
fn line<'a>(text: &'a str, label: &str) -> Result<&'a str, CandidateError> {
    let mut rows = text.lines().filter_map(|l| l.strip_prefix(label));
    let one = rows.next().ok_or(CandidateError::Malformed)?;
    if rows.next().is_some() {
        return Err(CandidateError::Malformed);
    }
    Ok(one)
}
pub fn verify_salted<L: Level>(
    key: &[u8],
    message: &[u8],
    signature: &[u8],
    bits: u32,
) -> Result<(), VerifyError> {
    let id = KeyIdentity::<L>::decode(key)?;
    let response = Response::<L>::decode(signature)?;
    let s = ChallengeScalar::derive_width(&id, message, response.salt, bits)
        .ok_or(VerifyError::Length)?;
    prism_verify::research::verify_scalar::<L>(key, signature, s.as_bytes(), bits)
}
