// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Field operations of one verification, per stage, and the verdict and
//! cost of every negative.
//!
//! ```text
//! cargo run --release --locked -p prism-verify --example op_counts \
//!     --features prism-field/op-count
//! ```
//!
//! `scripts/op_counts.sh` runs this and writes the two committed records.
//! Lines starting `COUNT` are `docs/benchmarks/op-counts.txt`; lines
//! starting `VERDICT` are `docs/benchmarks/negative-verdicts.tsv`. The
//! feature is given on the command line and by no manifest, so a build
//! without it exits with an error instead of printing zeros.
//!
//! A count is a call through `prism_field::Field` in the extension field
//! (`ext_*`) or made directly in the base field (`base_*`). Hashing and the
//! Baillie–PSW test do not use the field and are not in this metric.

#[path = "../tests/support/mod.rs"]
mod support;

use prism_field::ops::{self, Counts, Slot};
use prism_field::{ParameterSet, P248, P324, P376, P500};
use prism_verify::{verify, CommittedSet, Stage, VerifyError};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use support::identity::{identity_response, Form};
use support::{files, negatives, primary, set, vector};

fn fields(counts: Slot) -> String {
    let one = |name: &str, c: Counts| {
        format!(
            "{name}_mul={} {name}_square={} {name}_inv={} {name}_sqrt={} {name}_is_square={}",
            c.mul, c.square, c.inv, c.sqrt, c.is_square
        )
    };
    format!("{} {}", one("ext", counts.ext), one("base", counts.base))
}
fn total(table: &[Slot]) -> Slot {
    table.iter().fold(Slot::default(), |sum, slot| Slot {
        base: sum.base.plus(slot.base),
        ext: sum.ext.plus(slot.ext),
    })
}
fn counted<L: ParameterSet>(
    key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> (Result<(), VerifyError>, [Slot; ops::SLOTS]) {
    let _ = ops::take();
    let verdict = verify::<L>(key, message, signature);
    (verdict, ops::take())
}
fn name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}
fn stages(kind: &str, table: &[Slot]) {
    for stage in Stage::ALL {
        let slot = table[stage.slot()];
        if stage == Stage::Outside {
            assert_eq!(
                slot,
                Slot::default(),
                "a verification counted outside a stage"
            );
            continue;
        }
        println!("COUNT {kind} stage={} {}", stage.name(), fields(slot));
    }
    println!("COUNT {kind} stage=total {}", fields(total(table)));
}
fn accept<L: ParameterSet>(path: &Path, text: &str, per_stage: bool) -> Result<Slot, String> {
    let (key, message, response) = vector::<L>(text);
    let (verdict, table) = counted::<L>(&key, &message, &response.encode());
    if verdict != Ok(()) {
        return Err(format!("{}: honest vector gave {verdict:?}", name(path)));
    }
    let sum = total(&table);
    if sum.ext.mul + sum.ext.square == 0 {
        return Err(format!(
            "{}: an accepted signature counted nothing",
            name(path)
        ));
    }
    let kind = format!("accept set={} vector={}", L::NAME, name(path));
    if per_stage {
        stages(&kind, &table);
    } else {
        println!("COUNT {kind} stage=total {}", fields(sum));
    }
    Ok(sum)
}
/// Every negative of one primary vector: its verdict and total cost, the
/// per-stage counts of the first case of each refusal, and the dearest.
fn refuse<L: ParameterSet>(path: &Path, text: &str, accepted: Slot) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut dearest: Option<(u64, String, VerifyError)> = None;
    let mut most_inversions = 0;
    for case in negatives::<L>(text) {
        let (verdict, table) = counted::<L>(&case.key, &case.message, &case.signature);
        let error = match verdict {
            Err(e) if case.allowed.contains(&e) => e,
            other => return Err(format!("{} {}: {other:?}", name(path), case.name)),
        };
        let sum = total(&table);
        println!("{}", verdict_row::<L>(&name(path), &case.name, error, sum));
        if seen.insert(error) {
            let kind = format!(
                "refuse set={} vector={} case={} error={error:?}",
                L::NAME,
                name(path),
                case.name
            );
            stages(&kind, &table);
        }
        let products = sum.ext.mul + sum.ext.square;
        most_inversions = most_inversions.max(sum.ext.inv);
        if dearest.as_ref().is_none_or(|d| products > d.0) {
            dearest = Some((products, case.name, error));
        }
    }
    // Multiplications and squarings together are the comparable cost: a
    // change that turns one into the other moves neither refusal nor
    // acceptance. Inversions are compared on their own.
    let (products, case, error) = dearest.ok_or("no negatives")?;
    let accepted_products = accepted.ext.mul + accepted.ext.square;
    println!(
        "COUNT dearest-refusal set={} vector={} case={case} error={error:?} ext_mul_and_square={products} accept_ext_mul_and_square={accepted_products} most_ext_inv_of_any_refusal={most_inversions} accept_ext_inv={} above_accept={}",
        L::NAME,
        name(path),
        accepted.ext.inv,
        products > accepted_products || most_inversions > accepted.ext.inv
    );
    let missing: Vec<_> = [
        VerifyError::Length,
        VerifyError::Canonical,
        VerifyError::Origin,
        VerifyError::Singular,
        VerifyError::Sign,
        VerifyError::Model,
        VerifyError::Basis,
        VerifyError::Composite,
        VerifyError::Kernel,
        VerifyError::Pairing,
        VerifyError::Codomain,
        VerifyError::Image,
    ]
    .into_iter()
    .filter(|e| !seen.contains(e))
    .map(|e| format!("{e:?}"))
    .collect();
    println!(
        "COUNT refusals-without-a-vector set={} vector={} {}",
        L::NAME,
        name(path),
        if missing.is_empty() {
            "none".into()
        } else {
            missing.join(",")
        }
    );
    Ok(())
}
/// `p324` has no honest vector, so no accepted count and no negatives
/// derived from one. What can be counted is the refusal of a response
/// built from a public key alone that passes the pairing check
/// (`tests/support/identity.rs`): the key is the model curve of the Sage
/// file, under its refusal message (`prism_fixtures::p324`). The form refused as
/// `Codomain` is the first measured cost of the chain work at this prime.
/// It is the cost of a refusal, not of a verification that accepts.
fn refuse_at_p324() -> Result<(), String> {
    use prism_fixtures::p324::{curve_table, CURVE_FILE, MODEL_CURVE, REFUSAL_MESSAGE};
    let coefficient = curve_table()
        .curve(MODEL_CURVE)
        .coefficient
        .decode::<P324>();
    let message = REFUSAL_MESSAGE;
    let vector = format!("{CURVE_FILE}:{MODEL_CURVE}");
    for form in Form::ALL {
        let built = identity_response::<P324>(coefficient, message, form);
        let (verdict, table) = counted::<P324>(&built.key, message, &built.response.encode());
        let error = match verdict {
            Err(e) if form.allowed().contains(&e) => e,
            other => return Err(format!("{vector} {}: {other:?}", form.name())),
        };
        println!(
            "{}",
            verdict_row::<P324>(&vector, form.name(), error, total(&table))
        );
        let kind = format!(
            "refuse set={} vector={vector} case={} error={error:?}",
            P324::NAME,
            form.name()
        );
        stages(&kind, &table);
    }
    Ok(())
}
/// One row of `negative-verdicts.tsv`: a refusal and its total cost.
fn verdict_row<L: ParameterSet>(vector: &str, case: &str, error: VerifyError, sum: Slot) -> String {
    format!(
        "VERDICT\t{}\t{vector}\t{case}\t{error:?}\t{}\t{}\t{}\t{}\t{}",
        L::NAME,
        sum.ext.mul,
        sum.ext.square,
        sum.ext.inv,
        sum.ext.sqrt,
        sum.ext.is_square
    )
}
fn run() -> Result<(), String> {
    if !ops::ENABLED {
        return Err("built without counting: pass --features prism-field/op-count".into());
    }
    let primary = primary();
    for path in files() {
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let first = primary.contains(&path);
        let accepted = match set(&text) {
            CommittedSet::P248 => accept::<P248>(&path, &text, first),
            CommittedSet::P376 => accept::<P376>(&path, &text, first),
            CommittedSet::P500 => accept::<P500>(&path, &text, first),
        }?;
        if first {
            match set(&text) {
                CommittedSet::P248 => refuse::<P248>(&path, &text, accepted),
                CommittedSet::P376 => refuse::<P376>(&path, &text, accepted),
                CommittedSet::P500 => refuse::<P500>(&path, &text, accepted),
            }?;
        }
    }
    refuse_at_p324()
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("op_counts: {reason}");
            std::process::ExitCode::FAILURE
        }
    }
}
