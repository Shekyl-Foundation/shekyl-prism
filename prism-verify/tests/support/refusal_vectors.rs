// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The rows of `testdata/identity_refusals.tsv`, built here.
//!
//! At every set, the two responses of `identity.rs` under one curve as the
//! key. The file is what this module renders. A test holds the two equal,
//! and `examples/identity_refusals.rs` prints the rendering.
//!
//! The curve is one the operation counts already cover, so a row here and
//! the row of the same `set`, `vector` and `case` in
//! `docs/benchmarks/negative-verdicts.tsv` describe the same input:
//!
//! - at a set with a corpus, the response curve of the committed honest
//!   signature `prism_verify::reference::honest` returns, under that
//!   signature's message;
//! - at a set without one, the model curve of the set's Sage file, under
//!   the set's refusal message.
//!
//! The including file declares `identity` beside this module.

use std::fmt::Write as _;

use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_vectors::no_corpus::{curve_table, WithoutCorpus};
use prism_verify::reference::{honest, REFUSAL_VERDICTS};
use prism_verify::{CommittedSet, Response, Set, VerifyError};

use super::identity::{first_prime_salt_counter, identity_response, Form};

/// The file a set's honest timing sample is read from.
/// `prism_verify::reference::honest` reads the same three.
fn honest_file(set: CommittedSet) -> &'static str {
    match set {
        CommittedSet::P248 => "s5_lvl1.txt",
        CommittedSet::P376 => "s5_lvl3.txt",
        CommittedSet::P500 => "s5_lvl5.txt",
    }
}

/// One row, before it is text.
pub struct Row {
    pub set: Set,
    pub vector: String,
    pub form: Form,
    pub salt_counter: u64,
    pub message: Vec<u8>,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl Row {
    /// The one refusal the row's form may meet.
    pub fn error(&self) -> VerifyError {
        let [error] = self.form.allowed() else {
            panic!("an identity response has one pinned refusal");
        };
        *error
    }

    /// The text the file uses for the row's verdict.
    fn verdict(&self) -> &'static str {
        let error = self.error();
        REFUSAL_VERDICTS
            .into_iter()
            .find_map(|(text, verdict)| (verdict == error).then_some(text))
            .expect("the file has a text for each form's refusal")
    }

    fn render(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.set,
            self.vector,
            self.form.name(),
            self.verdict(),
            self.salt_counter,
            hex(&self.message),
            hex(&self.public_key),
            hex(&self.signature),
        )
    }
}

fn hex(bytes: &[u8]) -> String {
    let text = String::with_capacity(bytes.len() * 2);
    bytes.iter().fold(text, |mut text, byte| {
        write!(text, "{byte:02x}").expect("writing to a string");
        text
    })
}

/// Both forms under `coefficient` as the key.
fn both_forms<L: ParameterSet>(
    set: Set,
    vector: &str,
    coefficient: L::Ext,
    message: &[u8],
) -> [Row; 2] {
    Form::ALL.map(|form| {
        let built = identity_response::<L>(coefficient, message, form);
        Row {
            set,
            vector: vector.to_owned(),
            form,
            salt_counter: first_prime_salt_counter::<L>(&built.key, message),
            message: message.to_vec(),
            public_key: built.key,
            signature: built.response.encode(),
        }
    })
}

fn with_a_corpus<L: ParameterSet>(set: CommittedSet) -> [Row; 2] {
    let sample = honest(set).expect("a committed honest signature");
    let curve = Response::<L>::decode(&sample.signature)
        .expect("the committed signature decodes")
        .curve;
    let vector = format!("{}:E", honest_file(set));
    both_forms::<L>(set.set(), &vector, curve, sample.message)
}

fn without_a_corpus<L: WithoutCorpus>(set: Set) -> [Row; 2] {
    let fixtures = L::FIXTURES;
    let coefficient = curve_table::<L>()
        .curve(fixtures.model_curve)
        .coefficient
        .decode::<L>();
    let vector = format!("{}:{}", fixtures.curve_file, fixtures.model_curve);
    both_forms::<L>(set, &vector, coefficient, fixtures.refusal_message)
}

/// Every row, in the order of [`Set::ALL`], both forms of a set together.
pub fn rows() -> Vec<Row> {
    Set::ALL
        .into_iter()
        .flat_map(|set| match set {
            Set::P248 => with_a_corpus::<P248>(CommittedSet::P248),
            Set::P324 => without_a_corpus::<P324>(set),
            Set::P376 => with_a_corpus::<P376>(CommittedSet::P376),
            Set::P500 => with_a_corpus::<P500>(CommittedSet::P500),
            Set::P664 => without_a_corpus::<P664>(set),
        })
        .collect()
}

/// What stands above the rows.
const HEADER: &str = "\
# Responses built from public data alone that the verifier must refuse.
# Written by `cargo run --release --locked -p prism-verify --example identity_refusals`.
# Do not edit: a test holds this file to what that command prints.
#
# Nothing in this file is accepted. Each row is a public key, a message and
# a signature, and the one error a verifier must return for them. No secret
# key was used to make any row, and none is known for the keys of p324 and
# p664.
#
# How a row is made. Take a curve as the public key and the same curve as
# the response curve. Let (P, Q) be the basis of the 2^e-torsion the
# verifier derives from that key. The response points are
#   identity-response-q-negated      (P, -Q)      refused as Kernel
#   identity-response-q-difference   (P, P - Q)   refused as Codomain
# with both points negated where y(P) needs the other sign. Either pair has
# the pairing the verifier asks for, so both pass the pairing check. The
# first is refused at the start of the chain work. The second runs both
# chains and is refused where their codomains are compared. The salt is the
# little-endian salt_counter in the low eight bytes, zero above: the first
# counter from zero whose challenge degree is prime under the key and the
# message.
#
# The vector column names the curve.
#   s5_lvlN.txt:E      the response curve E of that committed honest
#                      signature, under that signature's message
#   pNNN_curves.tsv:K  curve K of that committed Sage file, the permitted
#                      model of its class, under the set's refusal message
# The row of the same set and case in docs/benchmarks/negative-verdicts.tsv
# (vector s5_lvlN.txt, or the same pNNN_curves.tsv:K) is the operation
# count of the same input.
#
# Columns are tab-separated. message, public_key and signature are hex. A
# signature is the response curve, x(P), y(P), x(Q), y(Q) and the salt, in
# that order, each field element in the set's canonical encoding.
set\tvector\tcase\tverdict\tsalt_counter\tmessage\tpublic_key\tsignature
";

/// The whole file.
pub fn file() -> String {
    rows().iter().fold(HEADER.to_owned(), |mut text, row| {
        text.push_str(&row.render());
        text.push('\n');
        text
    })
}
