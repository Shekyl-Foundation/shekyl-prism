// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Experimental Unix signer backed by the pinned, patched C implementation.
//! This is a research interface. The native ideal-to-isogeny implementation
//! has not been audited for secret-dependent timing or memory access.
//!
//! One [`Session`] keygens once and signs many messages. [`Signer`] is that
//! session at a level named at run time. The salt KDF and the native response
//! live here; a reduced-width experiment calls [`Session::search`] with its
//! own challenge and verifier.

#![forbid(unsafe_code)]

mod backend;

pub use backend::decimal;

use std::io::Read;
use std::path::{Path, PathBuf};

use prism_field::{Field, Level, Level1, Level3, Level5, Salt};
use prism_verify::{
    signature::canonical_sign, verify, ChallengeScalar, KeyIdentity, Nist, Response,
};
use sha3::{
    digest::{core_api::CoreWrapper, ExtendableOutput, Update, XofReader},
    CShake256, CShake256Core,
};
use zeroize::Zeroizing;

/// Sizes the signing call is willing to spend. The signature carries none of them.
pub mod limits {
    /// Bytes of the master seed and of every derived stream.
    pub const STREAM_LEN: usize = 48;
    /// Caller randomness `R` absorbed into each salt.
    pub const CALLER_RANDOMNESS_LEN: usize = 32;
    /// Counters one call may derive: `0..SALT_COUNTERS`. The wire has no counter.
    pub const SALT_COUNTERS: u64 = 65_536;
    /// Prime degrees one call may hand to the native backend.
    pub const PRIME_CANDIDATES: u32 = 16;
}

/// A secret seed. Deliberately has no Debug or serialization implementation.
pub struct SecretSeed(Zeroizing<[u8; limits::STREAM_LEN]>);
impl SecretSeed {
    pub fn from_bytes(bytes: [u8; limits::STREAM_LEN]) -> Self {
        Self(Zeroizing::new(bytes))
    }
    pub fn generate() -> std::io::Result<Self> {
        let mut bytes = Zeroizing::new([0u8; limits::STREAM_LEN]);
        std::fs::File::open("/dev/urandom")?.read_exact(&mut *bytes)?;
        Ok(Self(bytes))
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Backend,
    Malformed,
    Exhausted,
}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
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

/// A native candidate failed. The search counts these and tries the next salt.
#[derive(Debug)]
pub enum CandidateError {
    Backend,
    Malformed,
}

/// Local research counters. These are never serialized with a signature.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SigningReport {
    pub signature: Vec<u8>,
    pub hash_candidates: u32,
    pub prime_candidates: u32,
    pub backend_errors: u32,
    pub malformed_responses: u32,
    pub verification_rejections: u32,
}

/// What [`Session::search`] should do with one salt.
pub enum Attempt {
    /// The degree is not prime. The counter is spent; the backend is not.
    Composite,
    /// A prime degree, little-endian. Shorter than the row's scalar width is
    /// zero-extended for the backend. Longer aborts the search.
    Prime(Vec<u8>),
}

/// One keygen. The scratch directory lives as long as this session.
pub struct Session<'a, L: Level> {
    backend: backend::Backend,
    seed: &'a SecretSeed,
    public_key: Vec<u8>,
    level: core::marker::PhantomData<L>,
}

impl<'a, L: Level> Session<'a, L> {
    /// Keygen under `seed`. The backend must be this row's SHAKE signer.
    pub fn open(program: impl AsRef<Path>, seed: &'a SecretSeed) -> Result<Self, Error> {
        let backend = backend::Backend::open(program.as_ref())?;
        backend.check_version(L::E)?;
        let stream = stream(&seed.0, L::NAME, b"keygen", b"");
        let text = backend.keygen(&stream).map_err(Error::from)?;
        let public_key = decode_hex(line(&text, "A ")?)?;
        KeyIdentity::<L>::decode(&public_key).map_err(|_| Error::Malformed)?;
        Ok(Self {
            backend,
            seed,
            public_key,
            level: core::marker::PhantomData,
        })
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Salt for this counter. The same inputs always return the same salt.
    pub fn salt(
        &self,
        message: &[u8],
        randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
        counter: u64,
    ) -> L::Salt {
        salt_for::<L>(&self.seed.0, &self.public_key, message, randomness, counter)
    }

    /// Native response for `scalar` and `salt`, sign-normalized, not verified.
    ///
    /// `scalar` is little-endian and at most [`Level::S_LEN`] bytes. The
    /// backend receives it padded to that width. The caller verifies.
    pub fn candidate(&self, scalar: &[u8], salt: L::Salt) -> Result<Vec<u8>, CandidateError> {
        if scalar.len() > L::S_LEN {
            return Err(CandidateError::Malformed);
        }
        let mut canonical = scalar.to_vec();
        canonical.resize(L::S_LEN, 0);
        let stream = stream(&self.seed.0, L::NAME, b"sign", &canonical);
        let text = self
            .backend
            .sign_scalar(&backend::decimal(&canonical), &stream)?;
        let response = response_from_text::<L>(&text, salt)?;
        Ok(response.encode())
    }

    /// Full-width salted signing. A released signature has passed `verify`.
    pub fn sign(
        &self,
        message: &[u8],
        randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
    ) -> Result<SigningReport, Error> {
        let public_key = self.public_key.clone();
        let identity = KeyIdentity::<L>::decode(&public_key).map_err(|_| Error::Malformed)?;
        self.search(
            message,
            randomness,
            |salt| {
                let scalar = ChallengeScalar::derive(&identity, message, salt);
                if scalar.degree_is_prime::<L>() {
                    Attempt::Prime(scalar.as_bytes().to_vec())
                } else {
                    Attempt::Composite
                }
            },
            |signature| verify::<L>(&public_key, message, signature).is_ok(),
        )
    }

    /// Walk the salt counters. `challenge` classifies each salt. `accept`
    /// is the verifier for a normalized response. Prime attempts stop at
    /// [`limits::PRIME_CANDIDATES`]; the next counter then exhausts the search
    /// without another hash.
    pub fn search<F, V>(
        &self,
        message: &[u8],
        randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
        mut challenge: F,
        mut accept: V,
    ) -> Result<SigningReport, Error>
    where
        F: FnMut(L::Salt) -> Attempt,
        V: FnMut(&[u8]) -> bool,
    {
        let mut report = SigningReport::default();
        for counter in 0..limits::SALT_COUNTERS {
            if report.prime_candidates == limits::PRIME_CANDIDATES {
                return Err(Error::Exhausted);
            }
            report.hash_candidates += 1;
            let salt = self.salt(message, randomness, counter);
            let scalar = match challenge(salt) {
                Attempt::Composite => continue,
                Attempt::Prime(scalar) => scalar,
            };
            if scalar.len() > L::S_LEN {
                return Err(Error::Malformed);
            }
            report.prime_candidates += 1;
            let signature = match self.candidate(&scalar, salt) {
                Ok(signature) => signature,
                Err(CandidateError::Backend) => {
                    report.backend_errors += 1;
                    continue;
                }
                Err(CandidateError::Malformed) => {
                    report.malformed_responses += 1;
                    continue;
                }
            };
            if accept(&signature) {
                report.signature = signature;
                return Ok(report);
            }
            report.verification_rejections += 1;
        }
        Err(Error::Exhausted)
    }
}

/// The salt the signer would derive. `seed` is the master seed, not a stream.
pub fn salt_for<L: Level>(
    seed: &[u8; limits::STREAM_LEN],
    public_key: &[u8],
    message: &[u8],
    randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
    counter: u64,
) -> L::Salt {
    let salt_key = stream(seed, L::NAME, b"salt", b"");
    let mut hash: CShake256 = CoreWrapper::from_core(CShake256Core::new(b"shekyl-prism-salt-v1"));
    hash.update(salt_key.as_slice());
    hash.update(public_key);
    hash.update(randomness);
    hash.update(&counter.to_le_bytes());
    hash.update(message);
    let mut bytes = vec![0u8; L::SALT_LEN];
    XofReader::read(&mut hash.finalize_xof(), &mut bytes);
    L::Salt::from_bytes(&bytes).expect("the salt KDF writes the row length")
}

fn stream(
    seed: &[u8; limits::STREAM_LEN],
    level: &str,
    label: &[u8],
    context: &[u8],
) -> Zeroizing<[u8; limits::STREAM_LEN]> {
    let mut hash: CShake256 = CoreWrapper::from_core(CShake256Core::new(b"shekyl-prism-secret-v1"));
    hash.update(seed);
    hash.update(level.as_bytes());
    hash.update(&(label.len() as u64).to_le_bytes());
    hash.update(label);
    hash.update(context);
    let mut out = Zeroizing::new([0u8; limits::STREAM_LEN]);
    XofReader::read(&mut hash.finalize_xof(), &mut *out);
    out
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
        Ok(self.session()?.public_key().to_vec())
    }
    /// Reproducible research signing: caller randomness is all zeros.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        self.sign_with_randomness(message, &[0; limits::CALLER_RANDOMNESS_LEN])
    }
    /// The caller owns the entropy policy. All-zero randomness is supported.
    pub fn sign_with_randomness(
        &self,
        message: &[u8],
        randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
    ) -> Result<Vec<u8>, Error> {
        Ok(self.sign_report(message, randomness)?.signature)
    }
    /// Research diagnostics are local metadata, not part of the signature.
    /// Counts can reflect secret-dependent failures; do not publish wallet logs.
    pub fn sign_report(
        &self,
        message: &[u8],
        randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
    ) -> Result<SigningReport, Error> {
        self.session()?.sign(message, randomness)
    }
    fn session(&self) -> Result<Opened<'_>, Error> {
        match self.level {
            Nist::I => Ok(Opened::Level1(Session::<Level1>::open(
                &self.backend,
                &self.seed,
            )?)),
            Nist::III => Ok(Opened::Level3(Session::<Level3>::open(
                &self.backend,
                &self.seed,
            )?)),
            Nist::V => Ok(Opened::Level5(Session::<Level5>::open(
                &self.backend,
                &self.seed,
            )?)),
        }
    }
}

enum Opened<'a> {
    Level1(Session<'a, Level1>),
    Level3(Session<'a, Level3>),
    Level5(Session<'a, Level5>),
}
impl Opened<'_> {
    fn public_key(&self) -> &[u8] {
        match self {
            Self::Level1(session) => session.public_key(),
            Self::Level3(session) => session.public_key(),
            Self::Level5(session) => session.public_key(),
        }
    }
    fn sign(
        &self,
        message: &[u8],
        randomness: &[u8; limits::CALLER_RANDOMNESS_LEN],
    ) -> Result<SigningReport, Error> {
        match self {
            Self::Level1(session) => session.sign(message, randomness),
            Self::Level3(session) => session.sign(message, randomness),
            Self::Level5(session) => session.sign(message, randomness),
        }
    }
}

impl From<CandidateError> for Error {
    fn from(error: CandidateError) -> Self {
        match error {
            CandidateError::Backend => Self::Backend,
            CandidateError::Malformed => Self::Malformed,
        }
    }
}

fn response_from_text<L: Level>(text: &str, salt: L::Salt) -> Result<Response<L>, CandidateError> {
    let mut signature = Vec::new();
    for name in ["E ", "Px ", "Py ", "Qx ", "Qy "] {
        signature.extend(decode_hex(line(text, name)?).map_err(|_| CandidateError::Malformed)?);
    }
    signature.extend(salt.as_bytes());
    let mut response = Response::<L>::decode(&signature).map_err(|_| CandidateError::Malformed)?;
    if !canonical_sign::<L>(response.p.y) {
        response.p.y = response.p.y.negate();
        response.q.y = response.q.y.negate();
    }
    Ok(response)
}

fn line<'a>(text: &'a str, prefix: &str) -> Result<&'a str, CandidateError> {
    let mut rows = text.lines().filter_map(|row| row.strip_prefix(prefix));
    let value = rows.next().ok_or(CandidateError::Malformed)?;
    if rows.next().is_some() {
        return Err(CandidateError::Malformed);
    }
    Ok(value)
}

fn decode_hex(text: &str) -> Result<Vec<u8>, CandidateError> {
    if !text.len().is_multiple_of(2) {
        return Err(CandidateError::Malformed);
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            u8::from_str_radix(
                std::str::from_utf8(pair).map_err(|_| CandidateError::Malformed)?,
                16,
            )
            .map_err(|_| CandidateError::Malformed)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_field::Level;

    #[test]
    fn the_salt_kdf_binds_every_input_and_writes_the_row_length() {
        fn check<L: Level>() {
            let seed = [0x42; limits::STREAM_LEN];
            let key = L::ext_encode(L::Ext::from_u64(3));
            let randomness = [0u8; limits::CALLER_RANDOMNESS_LEN];
            let salt = salt_for::<L>(&seed, &key, b"message", &randomness, 0);
            assert_eq!(salt.as_bytes().len(), L::SALT_LEN);
            assert_ne!(salt, salt_for::<L>(&seed, &key, b"message", &randomness, 1));
            assert_ne!(salt, salt_for::<L>(&seed, &key, b"other", &randomness, 0));
            let mut flipped = randomness;
            flipped[0] ^= 1;
            assert_ne!(salt, salt_for::<L>(&seed, &key, b"message", &flipped, 0));
            let mut other_seed = seed;
            other_seed[0] ^= 1;
            assert_ne!(
                salt,
                salt_for::<L>(&other_seed, &key, b"message", &randomness, 0)
            );
        }
        check::<Level1>();
        check::<Level3>();
        check::<Level5>();
    }
}
