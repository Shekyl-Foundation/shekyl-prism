// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The committed PRISM test vectors, and a reader for each family.
//!
//! This crate owns every committed test file of the repository and is the
//! one place they are read. It depends on `prism-field` and on nothing
//! else of the workspace: it does not depend on a verifier, so another
//! implementation can take the data and the readers without taking one.
//! Values are given as bytes or as field elements. What a test builds
//! from them, a point or a response or an error, it builds itself.
//!
//! The library crates (`prism-field`, `prism-isogeny`, `prism-verify`)
//! name this crate as a dev-dependency only, so no build of the verifier
//! contains test data. Tools (the bench, the comparison, the fuzz
//! targets) depend on it normally.
//!
//! # Where the files are
//!
//! Under `data/`, in three directories, each a group of files that used
//! to sit beside the crate that tests against them:
//!
//! - `data/verify/`: signatures, keys and whole-verifier vectors;
//! - `data/isogeny/`: intermediate values of the isogeny arithmetic;
//! - `data/compare/`: the archive the protocol comparison replays.
//!
//! `data.sha256` lists the SHA-256 of every file. CI holds the files to
//! it, so a data file cannot change without that list changing in the
//! same commit.
//!
//! Small files are compiled into the crate as text. The honest-signature
//! corpus is read from disk, so its readers need a checkout of the
//! repository and say so.
//!
//! # The families
//!
//! ## Honest signatures and their keys: [`corpus`]
//!
//! - **What:** 113 signatures the verifier accepts, at the three sets
//!   that have a signer: six primary vectors (`data/verify/s5_lvl*.txt`,
//!   two keys per set) and 107 more in `data/verify/accept/`.
//! - **Format:** lines of `name value`. `level` is the set's opaque
//!   string, `message` the signed text, `A` the public key, and `E`,
//!   `Px`, `Py`, `Qx`, `Qy`, `salt` the signature, all in hex. The other
//!   lines are intermediate values.
//! - **From:** the research signer, which drives the C reference at the
//!   commit `docs/C_REFERENCE.md` pins, from the public seed streams of
//!   `scripts/fixtures/salt_seed_manifest.json`. These are vectors for a
//!   verifier, not known-answer tests of a production key derivation.
//! - **Regenerate:** `python3 scripts/regenerate_salted_vectors.py`. CI
//!   rebuilds them and requires no byte to change.
//!
//! ## Identity refusals: [`refusals`]
//!
//! - **What:** two responses at each of the five sets that pass the
//!   pairing check with no secret and must be refused, as `Kernel` and as
//!   `Codomain`. They are the only whole-verifier vectors at `p324` and
//!   `p664`.
//! - **Format:** `data/verify/identity_refusals.tsv`, tab-separated, with
//!   a header that says how a row is made.
//! - **From:** the verifier's own basis of a curve, built by
//!   `prism-testkit`.
//! - **Regenerate:**
//!   `cargo run --release --locked -p prism-testkit --example identity_refusals`.
//!   A test of that crate holds the file to that output.
//!
//! ## Sets without a corpus: [`no_corpus`], [`curve_table`]
//!
//! - **What:** at `p324` and `p664`, where no signer exists: field
//!   arithmetic (`<set>_field.tsv`), supersingular curves with points and
//!   pairings (`<set>_curves.tsv`), and challenge scalars
//!   (`<set>_challenge.tsv`), all in `data/verify/`.
//! - **Format:** tab-separated rows under a header that names the
//!   columns; [`curve_table`] reads the curve file by column name.
//! - **From:** the field and curve files are written by
//!   `scripts/sage/no_corpus_fixture.sage` in the pinned Sage image the
//!   script names; each file's header records the image and the digest
//!   of the script that wrote it. The challenge files are written by
//!   `scripts/challenge_scalar.py`, which has its own Keccak.
//! - **Regenerate:** `sage scripts/sage/no_corpus_fixture.sage <set>` in
//!   that image, and `python3 scripts/challenge_scalar.py --write`.
//!
//! ## C basis hints: [`hints`]
//!
//! - **What:** one C-generated public key per set with a corpus and the
//!   basis hint the C tree derived for it.
//! - **Format:** `data/verify/c_basis_hints.txt`, lines of
//!   `<level> <hex of A> <hint byte>` under `#` lines of provenance.
//! - **From:** the C reference at its pin, from a frozen seed.
//! - **Regenerate:** `python3 scripts/export_c_basis_hints.py`. CI
//!   rebuilds it and requires no byte to change.
//!
//! ## Weil pairing values: [`kat::WEIL_PAIRING`]
//!
//! - **What:** Sage's value of the two pairings the verifier compares,
//!   for each primary vector.
//! - **Format:** `data/verify/weil_pairing.tsv`, columns `file`, `vk`,
//!   `sig`.
//! - **From and regenerate:** `sage scripts/sage/weil_pairing.sage`, with
//!   the `Theta_dim4` library at its pinned commit.
//!
//! ## The Sage model: [`model`]
//!
//! - **What:** for each primary vector, the matrices, every null point of
//!   both chains and the step-8 images of the accept path, with the
//!   model's verdicts on the negatives and on the accept corpus.
//! - **Format:** JSON files and one TSV in `data/isogeny/s5_model/`, with
//!   `REPORT.md`, the model's own account of its runs.
//! - **From and regenerate:** `sage scripts/sage/s5_model.sage`, with the
//!   `Theta_dim4` library at the commit `REPORT.md` names.
//!
//! ## Residue known answers: [`kat::RESIDUE_KNOWN_ANSWERS`]
//!
//! - **What:** products and inverses modulo `2^e` at every set.
//! - **Format:** `data/isogeny/residue_known_answers.tsv`, rows
//!   `MUL set a b a*b` and `INV set s 1/s s^2 (1/s)^2`, tab-separated,
//!   values in little-endian hex.
//! - **From:** Python's integers and `pow(s, -1, 2**e)`.
//! - **Regenerate:** `python3 scripts/residue_known_answers.py --write`;
//!   CI runs `--check`.
//!
//! ## Chains and endomorphisms: [`kat`]
//!
//! - **What:** two theta chains and a fixed-degree isogeny at `p248`
//!   (`data/isogeny/chain_lvl1.txt`, `chain_lvl1_order.txt`,
//!   `e0_fixed_lvl1.txt`), and one endomorphism of the starting curve of
//!   the challenge degree per set with a corpus
//!   (`data/verify/e0_endomorphisms.txt`).
//! - **Format:** lines of `name value`, points as hex coordinates.
//! - **From:** the chain files were produced from the C reference's
//!   `theta_chain_compute_and_eval`; no script in this repository
//!   regenerates them. The endomorphisms are found by
//!   `scripts/export_e0_endomorphisms.py`.
//! - **Regenerate:** `python3 scripts/export_e0_endomorphisms.py` for the
//!   endomorphisms. The chain files have no generator here.
//!
//! ## The unsalted archive: [`unsalted`]
//!
//! - **What:** the 113 vectors as they were before the challenge took a
//!   salt. No verifier of the current protocol accepts them.
//! - **Format:** as the honest signatures, in `data/compare/unsalted/`.
//! - **From:** `scripts/regenerate_salted_vectors.py` archived them when
//!   it wrote the salted corpus. They are not regenerated.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::path::PathBuf;

/// The text of a committed file of the `verify` group.
macro_rules! verify_data {
    ($name:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/verify/", $name))
    };
}

/// The text of a committed file of the `isogeny` group.
macro_rules! isogeny_data {
    ($name:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/isogeny/", $name))
    };
}

pub mod corpus;
pub mod curve_table;
pub mod hex;
pub mod hints;
pub mod kat;
pub mod model;
pub mod no_corpus;
pub mod refusals;
pub mod unsalted;

/// One of the three directories the committed files are grouped in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    /// Signatures, keys and whole-verifier vectors.
    Verify,
    /// Intermediate values of the isogeny arithmetic.
    Isogeny,
    /// The archive the protocol comparison replays.
    Compare,
}

impl Group {
    /// All three.
    pub const ALL: [Self; 3] = [Self::Verify, Self::Isogeny, Self::Compare];

    /// The group's directory, relative to this crate.
    const fn relative(self) -> &'static str {
        match self {
            Self::Verify => "data/verify",
            Self::Isogeny => "data/isogeny",
            Self::Compare => "data/compare",
        }
    }

    /// The group's directory in a checkout of this repository. A tool
    /// that takes a file by path, or walks a directory, starts here.
    pub fn dir(self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(self.relative())
    }
}

#[cfg(test)]
mod tests {
    use super::Group;

    #[test]
    fn every_group_has_its_directory() {
        for group in Group::ALL {
            assert!(group.dir().is_dir(), "{group:?}");
        }
    }
}
