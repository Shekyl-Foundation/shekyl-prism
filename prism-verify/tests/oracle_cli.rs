// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Check the records the differential scripts actually receive from the binaries.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use prism_vectors::corpus::Primary;
use sha3::{Digest, Sha3_256};

fn run(binary: &str, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(binary)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    // A mutation can reject every coefficient in the deterministic sampler.
    // Reap the actual child so a hung oracle fails its test within the shard.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("oracle did not finish: {binary} {args:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn degree_stream_skips_comments_and_reports_both_verdicts() {
    let binary = env!("CARGO_BIN_EXE_degree_check");
    // 2^7 - 1 = 127 and 2^9 - 1 = 511 = 7 * 73.
    let rows = "# degrees\n\np248 4 01\np664 5 01\n";
    assert_eq!(
        stdout(run(binary, &[], rows)),
        "p248 4 01 prime\np664 5 01 composite\n"
    );
    let bad = run(binary, &[], "bad\n");
    assert!(!bad.status.success());
    assert!(String::from_utf8(bad.stderr).unwrap().contains("line 1:"));
}

#[test]
fn byte_oracle_emits_one_verdict_per_record_and_refuses_bad_records() {
    let binary = env!("CARGO_BIN_EXE_verify_bytes");
    let vector = Primary::Lvl1.vector();
    let level = vector.level().unwrap();
    let key = vector.field("A").unwrap();
    let message = hex(vector.message().unwrap());
    let signature = hex(&vector.signature().unwrap());
    let rows = format!("{level} {key} {message} {signature}\n{level} {key} - 00\n");
    assert_eq!(stdout(run(binary, &[], &rows)), "Ok\nLength\n");
    for fields in ["00 -", "xyz - 00", "00 0 00", "00 - gg"] {
        let bad = run(binary, &[], &format!("{level} {fields}\n"));
        assert!(!bad.status.success(), "{fields}");
        assert!(bad.stdout.is_empty(), "{fields}");
    }
}

#[test]
fn challenge_oracle_emits_the_committed_scalar_and_verdict() {
    let vector = Primary::Lvl1.vector();
    let args = ["level", "A", "message", "salt"].map(|field| vector.field(field).unwrap());
    let output = stdout(run(env!("CARGO_BIN_EXE_s5_challenge"), &args, ""));
    // int(s).to_bytes(31, "little").hex() from the pinned C vector s5_lvl1.txt.
    assert_eq!(
        output,
        "a32dad71f108ef91f1205b819d02ac5ae4c29885791cb727d211eda98f9312 prime\n"
    );
}

#[test]
fn basis_oracle_emits_the_committed_coordinates_in_wire_order() {
    let vector = Primary::Lvl1.vector();
    let output = stdout(run(
        env!("CARGO_BIN_EXE_even_basis"),
        &["p248", vector.field("A").unwrap()],
        "",
    ));
    let rows: Vec<_> = output.lines().collect();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows[..3],
        ["VkPx", "VkDx", "VkQx"].map(|f| vector.field(f).unwrap())
    );
    assert_eq!(
        prism_vectors::hex::decode(rows[3]).unwrap().len(),
        vector.public_key().unwrap().len()
    );
}

#[test]
fn sample_oracle_preserves_the_deterministic_coefficient_stream() {
    // Counters 11 and 13 are the first two with both 41-byte LE halves
    // below p = 3*2^324 - 1. Both have a basis hint.
    let expected: String = [11u64, 13]
        .into_iter()
        .map(|counter| {
            let mut bytes = Vec::new();
            for block in 0u64..3 {
                let mut hash = Sha3_256::new();
                hash.update(b"p324");
                hash.update(counter.to_le_bytes());
                hash.update(block.to_le_bytes());
                bytes.extend_from_slice(&hash.finalize());
            }
            format!("{}\n", hex(&bytes[..82]))
        })
        .collect();
    assert_eq!(
        stdout(run(
            env!("CARGO_BIN_EXE_even_basis"),
            &["sample", "p324", "2"],
            ""
        )),
        expected
    );
}
