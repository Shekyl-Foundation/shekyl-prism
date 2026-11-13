// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
//! The verdict of every file of a fuzz corpus, for comparing two builds.
//!
//! ```text
//! cargo run --release --locked -p prism-verify --example corpus_verdicts -- 1 fuzz/corpus/verify_level_1
//! ```
//!
//! The arguments are the level (1, 3 or 5) and a directory. Each file is
//! read the way the `verify_level_*` fuzz targets read their input: once
//! as key, message and signature all at once, and once split by the two
//! little-endian lengths in its first four bytes. One line per file, in
//! name order: the file name and the two verdicts. A behaviour-preserving
//! change gives the same output before and after.

use prism_field::{Level, Level1, Level3, Level5};
use prism_verify::{verify, VerifyError};
use std::path::PathBuf;

/// The split of `fuzz/fuzz_targets/verify_level_*.rs`.
fn split(data: &[u8]) -> (&[u8], &[u8], &[u8]) {
    if data.len() < 4 {
        return (data, b"", b"");
    }
    let public_key_len = usize::from(u16::from_le_bytes([data[0], data[1]]));
    let signature_len = usize::from(u16::from_le_bytes([data[2], data[3]]));
    let body = &data[4..];
    let public_key_len = public_key_len.min(body.len());
    let signature_len = signature_len.min(body.len() - public_key_len);
    (
        &body[..public_key_len],
        &body[public_key_len + signature_len..],
        &body[public_key_len..public_key_len + signature_len],
    )
}

fn show(verdict: Result<(), VerifyError>) -> String {
    match verdict {
        Ok(()) => "Ok".into(),
        Err(e) => format!("{e:?}"),
    }
}

fn run<L: Level>(files: &[PathBuf]) -> Result<(), String> {
    for path in files {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let whole = verify::<L>(&data, &data, &data);
        let (public_key, message, signature) = split(&data);
        let parts = verify::<L>(public_key, message, signature);
        println!(
            "{}\t{}\t{}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            show(whole),
            show(parts)
        );
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [level, directory] = arguments.as_slice() else {
        eprintln!("corpus_verdicts <level 1|3|5> <directory>");
        return std::process::ExitCode::from(2);
    };
    let mut files: Vec<PathBuf> = match std::fs::read_dir(directory) {
        Ok(entries) => entries.filter_map(|e| e.ok().map(|e| e.path())).collect(),
        Err(e) => {
            eprintln!("corpus_verdicts: {directory}: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    files.sort();
    if files.is_empty() {
        eprintln!("corpus_verdicts: {directory} holds no file");
        return std::process::ExitCode::FAILURE;
    }
    let outcome = match level.as_str() {
        "1" => run::<Level1>(&files),
        "3" => run::<Level3>(&files),
        "5" => run::<Level5>(&files),
        _ => Err("level is 1, 3 or 5".into()),
    };
    match outcome {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("corpus_verdicts: {reason}");
            std::process::ExitCode::FAILURE
        }
    }
}
