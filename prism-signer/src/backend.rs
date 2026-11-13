// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The pinned native signer process. One scratch directory, one secret file.

use std::os::unix::fs::DirBuilderExt;
use std::{
    fs::{self, DirBuilder},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use super::limits::STREAM_LEN;
use super::{CandidateError, Error};

pub(crate) struct Backend {
    program: PathBuf,
    secret: PathBuf,
    // Owns the directory. Drop removes it; nothing else reads it.
    _scratch: Scratch,
}

impl Backend {
    pub(crate) fn open(program: &Path) -> io::Result<Self> {
        let scratch = Scratch::new()?;
        let secret = scratch.path.join("secret");
        Ok(Self {
            program: program.to_path_buf(),
            secret,
            _scratch: scratch,
        })
    }

    /// The executable is the SHAKE backend for this row's exponent.
    pub(crate) fn check_version(&self, exponent: u32) -> Result<(), Error> {
        let version = Command::new(&self.program)
            .arg("backend-version")
            .output()?;
        if !version.status.success()
            || version.stdout != format!("prism-signer-shake256-v1 {exponent}\n").as_bytes()
        {
            return Err(Error::Backend);
        }
        Ok(())
    }

    pub(crate) fn keygen(&self, seed: &[u8; STREAM_LEN]) -> Result<String, CandidateError> {
        self.run(&["keygen".as_ref(), self.secret.as_os_str()], seed)
    }

    pub(crate) fn sign_scalar(
        &self,
        scalar: &str,
        seed: &[u8; STREAM_LEN],
    ) -> Result<String, CandidateError> {
        self.run(
            &["sign".as_ref(), self.secret.as_os_str(), scalar.as_ref()],
            seed,
        )
    }

    fn run(
        &self,
        args: &[&std::ffi::OsStr],
        seed: &[u8; STREAM_LEN],
    ) -> Result<String, CandidateError> {
        let mut child = Command::new(&self.program)
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
        let output = child
            .wait_with_output()
            .map_err(|_| CandidateError::Backend)?;
        if !output.status.success() {
            return Err(CandidateError::Backend);
        }
        String::from_utf8(output.stdout).map_err(|_| CandidateError::Malformed)
    }
}

struct Scratch {
    path: PathBuf,
}

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
                Ok(()) => return Ok(Self { path }),
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
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Decimal encoding of a little-endian scalar, for the native backend.
pub fn decimal(bytes: &[u8]) -> String {
    let mut digits = vec![0u16];
    for byte in bytes.iter().rev() {
        let mut carry = u16::from(*byte);
        for digit in &mut digits {
            let value = *digit * 256 + carry;
            *digit = value % 10;
            carry = value / 10;
        }
        while carry != 0 {
            digits.push(carry % 10);
            carry /= 10;
        }
    }
    digits
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + *digit as u8))
        .collect()
}
