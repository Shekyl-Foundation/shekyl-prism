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
use prism_field::{Level, Level1, Level3, Level5};
use prism_verify::{verify, Nist, Stage, VerifyError};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use support::{files, level, negatives, primary, vector};

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
fn counted<L: Level>(
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
fn accept<L: Level>(path: &Path, text: &str, per_stage: bool) -> Result<Slot, String> {
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
    let kind = format!("accept level={} vector={}", L::NAME, name(path));
    if per_stage {
        stages(&kind, &table);
    } else {
        println!("COUNT {kind} stage=total {}", fields(sum));
    }
    Ok(sum)
}
/// Every negative of one primary vector: its verdict and total cost, the
/// per-stage counts of the first case of each refusal, and the dearest.
fn refuse<L: Level>(path: &Path, text: &str, accepted: Slot) -> Result<(), String> {
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
        println!(
            "VERDICT\t{}\t{}\t{}\t{error:?}\t{}\t{}\t{}\t{}\t{}",
            L::NAME,
            name(path),
            case.name,
            sum.ext.mul,
            sum.ext.square,
            sum.ext.inv,
            sum.ext.sqrt,
            sum.ext.is_square
        );
        if seen.insert(error) {
            let kind = format!(
                "refuse level={} vector={} case={} error={error:?}",
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
        "COUNT dearest-refusal level={} vector={} case={case} error={error:?} ext_mul_and_square={products} accept_ext_mul_and_square={accepted_products} most_ext_inv_of_any_refusal={most_inversions} accept_ext_inv={} above_accept={}",
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
        "COUNT refusals-without-a-vector level={} vector={} {}",
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
fn run() -> Result<(), String> {
    if !ops::ENABLED {
        return Err("built without counting: pass --features prism-field/op-count".into());
    }
    let primary = primary();
    for path in files() {
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let first = primary.contains(&path);
        let accepted = match level(&text) {
            Nist::I => accept::<Level1>(&path, &text, first),
            Nist::III => accept::<Level3>(&path, &text, first),
            Nist::V => accept::<Level5>(&path, &text, first),
        }?;
        if first {
            match level(&text) {
                Nist::I => refuse::<Level1>(&path, &text, accepted),
                Nist::III => refuse::<Level3>(&path, &text, accepted),
                Nist::V => refuse::<Level5>(&path, &text, accepted),
            }?;
        }
    }
    Ok(())
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
