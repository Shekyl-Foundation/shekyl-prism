// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Known answers that are not signatures: a pairing table, residue
//! arithmetic, two theta chains and endomorphisms of the starting curve.
//!
//! Each is one committed file, given here as text. The tests that use
//! them compare their own intermediate values, so they read the lines
//! themselves; the crate documentation gives each format.

/// `weil_pairing.tsv`: Sage's values of the two Weil pairings the
/// verifier compares, for each primary vector. Columns `file`, `vk`,
/// `sig`, tab-separated; the values are the field's canonical encoding in
/// hex.
pub const WEIL_PAIRING: &str = verify_data!("weil_pairing.tsv");

/// `e0_endomorphisms.txt`: for each set with a corpus, one endomorphism
/// of the starting curve of the challenge degree, as a block headed by
/// the set's level string.
pub const E0_ENDOMORPHISMS: &str = verify_data!("e0_endomorphisms.txt");

/// `residue_known_answers.tsv`: products and inverses modulo `2^e` from
/// Python's integers, at every set. Rows are `MUL set a b a*b` and
/// `INV set s 1/s s^2 (1/s)^2`, the values little-endian hex.
pub const RESIDUE_KNOWN_ANSWERS: &str = isogeny_data!("residue_known_answers.tsv");

/// `chain_lvl1.txt`: a theta chain of length 160 at `p248` on the graph
/// of an endomorphism of the starting curve, from the C reference, with
/// its codomain and an image.
pub const CHAIN_LVL1: &str = isogeny_data!("chain_lvl1.txt");

/// `chain_lvl1_order.txt`: the same chain with the kernel points in the
/// other order.
pub const CHAIN_LVL1_ORDER: &str = isogeny_data!("chain_lvl1_order.txt");

/// `e0_fixed_lvl1.txt`: a fixed-degree isogeny from the starting curve at
/// `p248`, from the C reference.
pub const E0_FIXED_LVL1: &str = isogeny_data!("e0_fixed_lvl1.txt");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Primary;

    #[test]
    fn the_files_are_the_ones_named() {
        let mut weil = WEIL_PAIRING.lines();
        assert_eq!(weil.next(), Some("file\tvk\tsig"));
        assert_eq!(
            weil.filter(|line| !line.is_empty()).count(),
            Primary::ALL.len()
        );
        assert!(RESIDUE_KNOWN_ANSWERS.contains("\nMUL\tp664\t"));
        assert!(E0_ENDOMORPHISMS.starts_with("NIST-I\n"));
        assert!(CHAIN_LVL1.starts_with("length 160 "));
        assert!(CHAIN_LVL1_ORDER.starts_with("length 160 "));
        assert!(E0_FIXED_LVL1.starts_with("length "));
    }
}
