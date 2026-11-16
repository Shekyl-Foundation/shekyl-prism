// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! One verification, for a memory profiler: of one committed signature,
//! or of one response the verifier must refuse after its chain work.
//!
//! ```text
//! cargo build --release --locked -p prism-verify --example verify_once
//! target/release/examples/verify_once prism-vectors/data/verify/s5_lvl1.txt verify
//! target/release/examples/verify_once p664 refuse
//! ```
//!
//! The first argument is a vector file or the name of a set that has no
//! vectors. The second is one of four modes, in two pairs:
//!
//! - `load` reads and decodes the vector and stops; `verify` then verifies
//!   it once and exits non-zero unless it is accepted.
//! - `build` builds the response `(P, P − Q)` of `tests/support/identity.rs`
//!   and stops; `refuse` then verifies it once and exits non-zero unless
//!   it is refused as `Codomain`. With a vector file the key is that
//!   signature's response curve and the message is the vector's; with a
//!   set name the key is the model curve of the set's Sage file under the
//!   set's refusal message. Either way it is the input the count records
//!   call `identity-response-q-difference`.
//!
//! The difference between the two modes of a pair under a heap profiler is
//! what one verification holds. `scripts/peak_heap.sh` runs both pairs
//! under Valgrind's Massif. The second pair is the cost of a refusal: it
//! runs both chains and stops at the codomain comparison, short of the
//! image comparison an accepted verification goes on to.

#[path = "../tests/support/mod.rs"]
#[allow(dead_code)]
mod support;

use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_vectors::no_corpus::{curve_table, WithoutCorpus};
use prism_verify::{verify, CommittedSet, Set, VerifyError};
use std::hint::black_box;
use std::process::ExitCode;
use support::identity::{identity_response, Form};
use support::{set, vector};

fn run<L: ParameterSet>(text: &str, check: bool) -> bool {
    let (key, message, response) = vector::<L>(text);
    let signature = response.encode();
    let inputs = black_box((&key, &message, &signature));
    !check || verify::<L>(inputs.0, inputs.1, inputs.2).is_ok()
}

/// Build the response of the `Codomain` refusal under `coefficient` as a
/// key, and verify it when `check`.
fn refuse<L: ParameterSet>(coefficient: L::Ext, message: &[u8], check: bool) -> bool {
    let built = identity_response::<L>(coefficient, message, Form::Difference);
    let signature = built.response.encode();
    let inputs = black_box((&built.key, message, &signature));
    !check || verify::<L>(inputs.0, inputs.1, inputs.2) == Err(VerifyError::Codomain)
}

/// At a set with vectors: the key is the vector's response curve.
fn refuse_at_a_vector<L: ParameterSet>(text: &str, check: bool) -> bool {
    let (_, message, response) = vector::<L>(text);
    refuse::<L>(response.curve, &message, check)
}

/// At a set without vectors: the key is the model curve of its Sage file.
fn refuse_without_a_corpus<L: WithoutCorpus>(check: bool) -> bool {
    let fixtures = L::FIXTURES;
    let coefficient = curve_table::<L>()
        .curve(fixtures.model_curve)
        .coefficient
        .decode::<L>();
    refuse::<L>(coefficient, fixtures.refusal_message, check)
}

/// What the second argument asks for.
#[derive(Clone, Copy)]
enum Mode {
    Load,
    Verify,
    Build,
    Refuse,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args.as_slice() {
        [_, mode] if mode == "load" => Mode::Load,
        [_, mode] if mode == "verify" => Mode::Verify,
        [_, mode] if mode == "build" => Mode::Build,
        [_, mode] if mode == "refuse" => Mode::Refuse,
        _ => {
            eprintln!(
                "usage: verify_once <vector file | set without vectors> load|verify|build|refuse"
            );
            return ExitCode::from(2);
        }
    };
    // A set name, for a set that has no vector file to give.
    if let Ok(named) = args[0].parse::<Set>() {
        let check = match mode {
            Mode::Build => false,
            Mode::Refuse => true,
            Mode::Load | Mode::Verify => {
                eprintln!("{named}: a set is named for build or refuse; load and verify take a vector file");
                return ExitCode::from(2);
            }
        };
        let done = match named {
            Set::P324 => refuse_without_a_corpus::<P324>(check),
            Set::P664 => refuse_without_a_corpus::<P664>(check),
            Set::P248 | Set::P376 | Set::P500 => {
                eprintln!("{named} has vectors: give one of its vector files");
                return ExitCode::from(2);
            }
        };
        return if done {
            ExitCode::SUCCESS
        } else {
            eprintln!("{named}: not refused as Codomain");
            ExitCode::FAILURE
        };
    }
    let text = match std::fs::read_to_string(&args[0]) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{}: {e}", args[0]);
            return ExitCode::from(2);
        }
    };
    let (done, expected) = match mode {
        Mode::Load | Mode::Verify => {
            let check = matches!(mode, Mode::Verify);
            let accepted = match set(&text) {
                CommittedSet::P248 => run::<P248>(&text, check),
                CommittedSet::P376 => run::<P376>(&text, check),
                CommittedSet::P500 => run::<P500>(&text, check),
            };
            (accepted, "accepted")
        }
        Mode::Build | Mode::Refuse => {
            let check = matches!(mode, Mode::Refuse);
            let refused = match set(&text) {
                CommittedSet::P248 => refuse_at_a_vector::<P248>(&text, check),
                CommittedSet::P376 => refuse_at_a_vector::<P376>(&text, check),
                CommittedSet::P500 => refuse_at_a_vector::<P500>(&text, check),
            };
            (refused, "refused as Codomain")
        }
    };
    if done {
        ExitCode::SUCCESS
    } else {
        eprintln!("{}: not {expected}", args[0]);
        ExitCode::FAILURE
    }
}
