// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! One verification of one committed signature, for a memory profiler.
//!
//! ```text
//! cargo build --release --locked -p prism-verify --example verify_once
//! target/release/examples/verify_once prism-verify/testdata/s5_lvl1.txt verify
//! ```
//!
//! The arguments are a vector file and `load` or `verify`. `load` reads
//! and decodes the vector and stops; `verify` then verifies it once and
//! exits non-zero unless it is accepted. The difference between the two
//! under a heap profiler is what one verification holds.
//! `scripts/peak_heap.sh` runs both under Valgrind's Massif.

#[path = "../tests/support/mod.rs"]
#[allow(dead_code)]
mod support;

use prism_field::{Level, Level1, Level3, Level5};
use prism_verify::{verify, Nist};
use std::hint::black_box;
use std::process::ExitCode;
use support::{level, vector};

fn run<L: Level>(text: &str, check: bool) -> bool {
    let (key, message, response) = vector::<L>(text);
    let signature = response.encode();
    let inputs = black_box((&key, &message, &signature));
    !check || verify::<L>(inputs.0, inputs.1, inputs.2).is_ok()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = match args.as_slice() {
        [_, mode] if mode == "load" => false,
        [_, mode] if mode == "verify" => true,
        _ => {
            eprintln!("usage: verify_once <vector file> load|verify");
            return ExitCode::from(2);
        }
    };
    let text = match std::fs::read_to_string(&args[0]) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{}: {e}", args[0]);
            return ExitCode::from(2);
        }
    };
    let accepted = match level(&text) {
        Nist::I => run::<Level1>(&text, check),
        Nist::III => run::<Level3>(&text, check),
        Nist::V => run::<Level5>(&text, check),
    };
    if accepted {
        ExitCode::SUCCESS
    } else {
        eprintln!("{}: not accepted", args[0]);
        ExitCode::FAILURE
    }
}
