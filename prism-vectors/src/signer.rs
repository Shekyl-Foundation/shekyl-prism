// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The signer's constants: what `docs/SIGNER_SPEC.md` section 1 defines,
//! generated at the five sets, and the values of two other code bases
//! they are compared with.
//!
//! No signer exists yet. These files are the committed output of
//! `scripts/signer_constants.py`, which computes each value from its
//! definition, and the data a later signer crate will hold its own copy
//! equal to. They are test data like the rest of this crate: no library
//! build contains them.
//!
//! # The files, in `data/signer/`
//!
//! - `<set>.txt`, one for each set ([`set`]): the starting basis of
//!   `E0[2^e]` and the matrices of the maximal order's generators on it,
//!   the exponent of the translation's chain, and the prime norm key
//!   generation first samples. Those rows are shipped to the signer.
//!   After them, the matrices of `i`, of the Frobenius and of their
//!   product, which exist to test the others, and a record of how the
//!   basis search ran.
//! - `shared.txt` ([`shared`]): what does not depend on the prime. The
//!   list of small primes the norm equation divides by, and four bounds
//!   of the quaternion layer. **The four bounds are provisional at the
//!   sets [`Shared::provisional_at`] names** (ledger O26 and O29): they
//!   are the C reference's constants, which that code has at three
//!   primes only.
//! - `oracle_c_reference.txt` and `oracle_sqisign_rs.txt` ([`oracle`]):
//!   the same values as two other code bases have them, extracted by the
//!   same script, each file with the repository, the commit and the
//!   digests of the files it was read from. The generator does not read
//!   them. They are held equal to the generated values by
//!   `signer_constants.py --check`.
//!
//! # Encodings
//!
//! A field element is the set's canonical encoding: the real part, then
//! the imaginary part, each little-endian. An integer modulo `2^e`, and
//! the secret norm, are little-endian bytes. A [`Matrix`] `M` of an
//! endomorphism `α` has `α(P0) = [m11]P0 + [m21]Q0` and
//! `α(Q0) = [m12]P0 + [m22]Q0`: its columns are the images.
//!
//! # Regenerate
//!
//! `python3 scripts/signer_constants.py --write`; CI runs `--check`. The
//! oracle files are rewritten by `--extract-c --write` from the pinned
//! submodule and by `--extract-sqisign-rs <checkout> --write`.

use std::sync::OnceLock;

use crate::hex;

/// The text of a committed file of the `signer` group.
macro_rules! signer_data {
    ($name:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/signer/", $name))
    };
}

/// Each set's file, by the set's name, in the order of the sets.
const SET_TEXTS: [(&str, &str); 5] = [
    ("p248", signer_data!("p248.txt")),
    ("p324", signer_data!("p324.txt")),
    ("p376", signer_data!("p376.txt")),
    ("p500", signer_data!("p500.txt")),
    ("p664", signer_data!("p664.txt")),
];

/// `shared.txt`, as text.
pub const SHARED_TEXT: &str = signer_data!("shared.txt");

/// Why a file of this group could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A line is not `<name> <value>`, a name is there twice or not at
    /// all, or a value is not in its row's encoding.
    Malformed,
    /// No file is of this set.
    UnknownSet,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Malformed => "a signer-constants file is not in its format",
            Self::UnknownSet => "no signer-constants file is of this set",
        })
    }
}

impl std::error::Error for Error {}

/// A `2×2` matrix modulo `2^e`, each entry little-endian bytes. The
/// module documentation says which entry is which.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matrix {
    /// The coefficient of `P0` in the image of `P0`.
    pub m11: Vec<u8>,
    /// The coefficient of `P0` in the image of `Q0`.
    pub m12: Vec<u8>,
    /// The coefficient of `Q0` in the image of `P0`.
    pub m21: Vec<u8>,
    /// The coefficient of `Q0` in the image of `Q0`.
    pub m22: Vec<u8>,
}

/// What became of one candidate of the basis search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Candidate {
    /// It is the first point.
    First,
    /// It is the second point.
    Second,
    /// It was refused: its point of order 2 is the first point's.
    Dependent,
}

/// One set's constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetConstants {
    /// The set's name, such as `p248`.
    pub name: &'static str,
    /// The exponent `e` of `p = f·2^e − 1`.
    pub e: u32,
    /// The cofactor `f`.
    pub f: u64,
    /// `x(P0)`. Shipped.
    pub basis_px: Vec<u8>,
    /// `x(Q0)`. Shipped.
    pub basis_qx: Vec<u8>,
    /// `x(P0 − Q0)`. Shipped.
    pub basis_pmqx: Vec<u8>,
    /// The action of the order's second generator, `i`. Shipped.
    pub action_gen2: Matrix,
    /// The action of `(i + j)/2`. Shipped.
    pub action_gen3: Matrix,
    /// The action of `(1 + k)/2`. Shipped.
    pub action_gen4: Matrix,
    /// The exponent of the translation's chain, `e − 2`. Shipped.
    pub chain_exponent: u32,
    /// The prime norm key generation first samples, little-endian.
    /// Shipped.
    pub secret_norm: Vec<u8>,
    /// The action of `i: (x, y) ↦ (−x, i·y)`. A test intermediate.
    pub action_i: Matrix,
    /// The action of the Frobenius. A test intermediate.
    pub action_j: Matrix,
    /// The action of `k = i∘j`. A test intermediate.
    pub action_k: Matrix,
    /// Each candidate `m` of the basis search whose point has order
    /// divisible by `2^e`, and what became of it.
    pub basis_search: Vec<(u32, Candidate)>,
    /// Which case of the rule's last step gave the basis: 2 or 3.
    pub basis_branch: u8,
    /// The odd part of the order of each point of [`Self::basis_search`].
    pub basis_odd_parts: Vec<u64>,
}

/// What does not depend on the prime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shared {
    /// 2 and the first primes that are 1 modulo 4, increasing.
    pub small_primes: Vec<u16>,
    /// Rounds of the quaternion layer's primality test. Provisional.
    pub primality_rounds: u32,
    /// Half-width of the box searched for an equivalent ideal of prime
    /// norm. Provisional.
    pub equivalent_ideal_box: u32,
    /// Bit length of the coefficients a generator of an ideal is drawn
    /// from. Provisional.
    pub generator_bits: u32,
    /// Multiples of one generator tried before another is drawn.
    /// Provisional.
    pub steps_per_generator: u32,
    /// The sets at which the four bounds are provisional.
    pub provisional_at: Vec<&'static str>,
}

/// The rows of a file: `<name> <value>` lines, `#` lines dropped. A name
/// twice is a fault.
fn rows(text: &'static str) -> Result<Vec<(&'static str, &'static str)>, Error> {
    let mut found: Vec<(&str, &str)> = Vec::new();
    for line in text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (name, value) = line.split_once(' ').ok_or(Error::Malformed)?;
        if found.iter().any(|(seen, _)| *seen == name) {
            return Err(Error::Malformed);
        }
        found.push((name, value));
    }
    Ok(found)
}

fn value(rows: &[(&'static str, &'static str)], name: &str) -> Result<&'static str, Error> {
    rows.iter()
        .find(|(row, _)| *row == name)
        .map(|(_, value)| *value)
        .ok_or(Error::Malformed)
}

fn number<T: core::str::FromStr>(text: &str) -> Result<T, Error> {
    text.parse().map_err(|_| Error::Malformed)
}

fn numbers<T: core::str::FromStr>(text: &str) -> Result<Vec<T>, Error> {
    text.split(' ').map(number).collect()
}

fn bytes(text: &str) -> Result<Vec<u8>, Error> {
    hex::decode(text).ok_or(Error::Malformed)
}

fn matrix(text: &str) -> Result<Matrix, Error> {
    let mut entries = text.split(' ').map(bytes);
    let mut next = || -> Result<Vec<u8>, Error> { entries.next().ok_or(Error::Malformed)? };
    let matrix = Matrix {
        m11: next()?,
        m12: next()?,
        m21: next()?,
        m22: next()?,
    };
    match entries.next() {
        None => Ok(matrix),
        Some(_) => Err(Error::Malformed),
    }
}

fn candidate(text: &str) -> Result<(u32, Candidate), Error> {
    let (m, role) = text.split_once(':').ok_or(Error::Malformed)?;
    let role = match role {
        "P0" => Candidate::First,
        "Q0" => Candidate::Second,
        "dependent" => Candidate::Dependent,
        _ => return Err(Error::Malformed),
    };
    Ok((number(m)?, role))
}

fn parse_set(text: &'static str) -> Result<SetConstants, Error> {
    let rows = rows(text)?;
    let get = |name| value(&rows, name);
    Ok(SetConstants {
        name: get("set")?,
        e: number(get("e")?)?,
        f: number(get("f")?)?,
        basis_px: bytes(get("basis_px")?)?,
        basis_qx: bytes(get("basis_qx")?)?,
        basis_pmqx: bytes(get("basis_pmqx")?)?,
        action_gen2: matrix(get("action_gen2")?)?,
        action_gen3: matrix(get("action_gen3")?)?,
        action_gen4: matrix(get("action_gen4")?)?,
        chain_exponent: number(get("chain_exponent")?)?,
        secret_norm: bytes(get("secret_norm")?)?,
        action_i: matrix(get("action_i")?)?,
        action_j: matrix(get("action_j")?)?,
        action_k: matrix(get("action_k")?)?,
        basis_search: get("basis_search")?
            .split(' ')
            .map(candidate)
            .collect::<Result<_, _>>()?,
        basis_branch: number(get("basis_branch")?)?,
        basis_odd_parts: numbers(get("basis_odd_parts")?)?,
    })
}

fn parse_shared(text: &'static str) -> Result<Shared, Error> {
    let rows = rows(text)?;
    let get = |name| value(&rows, name);
    let small_primes: Vec<u16> = numbers(get("small_primes")?)?;
    if number::<usize>(get("small_primes_length")?)? != small_primes.len() {
        return Err(Error::Malformed);
    }
    Ok(Shared {
        small_primes,
        primality_rounds: number(get("primality_rounds")?)?,
        equivalent_ideal_box: number(get("equivalent_ideal_box")?)?,
        generator_bits: number(get("generator_bits")?)?,
        steps_per_generator: number(get("steps_per_generator")?)?,
        provisional_at: get("provisional_at")?.split(' ').collect(),
    })
}

/// Every set's constants, in the order of the sets.
pub fn sets() -> Result<&'static [SetConstants], Error> {
    static SETS: OnceLock<Result<Vec<SetConstants>, Error>> = OnceLock::new();
    SETS.get_or_init(|| {
        SET_TEXTS
            .iter()
            .map(|(name, text)| {
                let set = parse_set(text)?;
                if set.name == *name {
                    Ok(set)
                } else {
                    Err(Error::Malformed)
                }
            })
            .collect()
    })
    .as_deref()
    .map_err(|err| *err)
}

/// The constants of the set named `name`, such as `p248`.
pub fn set(name: &str) -> Result<&'static SetConstants, Error> {
    sets()?
        .iter()
        .find(|set| set.name == name)
        .ok_or(Error::UnknownSet)
}

/// The constants that do not depend on the prime.
pub fn shared() -> Result<&'static Shared, Error> {
    static SHARED: OnceLock<Result<Shared, Error>> = OnceLock::new();
    SHARED
        .get_or_init(|| parse_shared(SHARED_TEXT))
        .as_ref()
        .map_err(|err| *err)
}

pub mod oracle {
    //! The values two other code bases have for the same constants.
    //!
    //! A line is `<set>.<row> <value>`. A row with the name of a row of a
    //! set's file is in that row's encoding, so the two texts compare
    //! equal; the C reference's file has further rows, which
    //! `scripts/signer_constants.py --check` tests. The `#` lines at the
    //! head of each file are its provenance: the repository, the commit,
    //! the licence, and the SHA-256 of every file a value was read from.
    //!
    //! Nothing here is generated by this repository.

    use super::Error;

    /// One of the two code bases.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum Source {
        /// The C reference at its pin: values at `p248`, `p376`, `p500`.
        CReference,
        /// `sqisign-rs`: values at `p324`, `p500`, `p664`.
        SqisignRs,
    }

    impl Source {
        /// Both.
        pub const ALL: [Self; 2] = [Self::CReference, Self::SqisignRs];

        /// The committed file, as text.
        pub const fn text(self) -> &'static str {
            match self {
                Self::CReference => signer_data!("oracle_c_reference.txt"),
                Self::SqisignRs => signer_data!("oracle_sqisign_rs.txt"),
            }
        }

        /// The value of `row` at the set named `set`, as the file has
        /// it. `Ok(None)` when the file has no such row.
        pub fn value(self, set: &str, row: &str) -> Result<Option<&'static str>, Error> {
            for (name, value) in super::rows(self.text())? {
                let (of_set, of_row) = name.split_once('.').ok_or(Error::Malformed)?;
                if of_set == set && of_row == row {
                    return Ok(Some(value));
                }
            }
            Ok(None)
        }

        /// The commit the file says it was read at.
        pub fn commit(self) -> Option<&'static str> {
            self.text()
                .lines()
                .find_map(|line| line.strip_prefix("# commit "))
        }
    }
}

#[cfg(test)]
mod tests {
    use prism_field::{ParameterSet, P248, P324, P376, P500, P664};

    use super::oracle::Source;
    use super::*;

    fn sizes<L: ParameterSet>() -> (&'static str, u32, u64, usize) {
        (L::NAME, L::E, L::COFACTOR.value(), L::FP2_LEN)
    }

    #[test]
    fn every_set_has_its_file_with_values_of_the_sets_sizes() {
        let expected = [
            sizes::<P248>(),
            sizes::<P324>(),
            sizes::<P376>(),
            sizes::<P500>(),
            sizes::<P664>(),
        ];
        let sets = sets().expect("the committed files parse");
        assert_eq!(sets.len(), expected.len());
        for (set, (name, e, f, fp2_len)) in sets.iter().zip(expected) {
            assert_eq!((set.name, set.e, set.f), (name, e, f));
            assert_eq!(set.chain_exponent, e - 2, "{name}");
            for element in [&set.basis_px, &set.basis_qx, &set.basis_pmqx] {
                assert_eq!(element.len(), fp2_len, "{name}");
            }
            let entry = (e as usize).div_ceil(8);
            for matrix in [
                &set.action_gen2,
                &set.action_gen3,
                &set.action_gen4,
                &set.action_i,
                &set.action_j,
                &set.action_k,
            ] {
                for entry_bytes in [&matrix.m11, &matrix.m12, &matrix.m21, &matrix.m22] {
                    assert_eq!(entry_bytes.len(), entry, "{name}");
                }
            }
            assert_eq!(set.action_gen2, set.action_i, "{name}");
            assert_eq!(set.basis_search.len(), set.basis_odd_parts.len(), "{name}");
            assert!(matches!(set.basis_branch, 2 | 3), "{name}");
            assert!(std::ptr::eq(super::set(name).unwrap(), set));
        }
        assert_eq!(super::set("p999"), Err(Error::UnknownSet));
    }

    #[test]
    fn the_shared_file_has_the_list_and_marks_the_bounds_provisional() {
        let shared = shared().expect("the committed file parses");
        assert_eq!(shared.small_primes.len(), 101);
        assert_eq!(shared.small_primes[..4], [2, 5, 13, 17]);
        assert_eq!(shared.small_primes.last(), Some(&1237));
        assert_eq!(
            (
                shared.primality_rounds,
                shared.equivalent_ideal_box,
                shared.generator_bits,
                shared.steps_per_generator
            ),
            (32, 64, 17, 20_000)
        );
        assert_eq!(shared.provisional_at, ["p324", "p664"]);
        assert!(SHARED_TEXT.contains("PROVISIONAL"));
    }

    #[test]
    fn an_oracle_file_names_its_commit_and_has_rows_in_the_sets_encoding() {
        assert_eq!(
            Source::CReference.commit(),
            Some("87fad7da5f12f79112521677f4f5530672afe9ba")
        );
        assert_eq!(
            Source::SqisignRs.commit(),
            Some("7a4a01bb8108f4d0c0726112cc7d3eaef8053439")
        );
        // The rows the two files share with a set's file are that file's
        // text: `p500` is the set both code bases have.
        let text = SET_TEXTS[3].1;
        for source in Source::ALL {
            for row in ["basis_px", "basis_qx", "basis_pmqx", "action_gen3"] {
                let theirs = source.value("p500", row).unwrap().expect("the row");
                assert!(text.contains(&format!("\n{row} {theirs}\n")), "{row}");
            }
            assert_eq!(source.value("p500", "no_such_row"), Ok(None));
        }
        assert_eq!(Source::CReference.value("p324", "basis_px"), Ok(None));
        assert_eq!(Source::SqisignRs.value("p248", "basis_px"), Ok(None));
    }

    #[test]
    fn a_file_out_of_its_format_is_refused() {
        assert_eq!(rows("norow\n").unwrap_err(), Error::Malformed);
        assert_eq!(rows("a 1\na 2\n").unwrap_err(), Error::Malformed);
        assert_eq!(matrix("00 01 02"), Err(Error::Malformed));
        assert_eq!(matrix("00 01 02 03 04"), Err(Error::Malformed));
        assert_eq!(matrix("00 01 02 0g"), Err(Error::Malformed));
        assert!(matrix("00 01 02 03").is_ok());
        assert_eq!(candidate("9:P1"), Err(Error::Malformed));
        assert_eq!(candidate("9"), Err(Error::Malformed));
        assert_eq!(candidate("19:dependent"), Ok((19, Candidate::Dependent)));
        assert_eq!(parse_set("set p248\n").unwrap_err(), Error::Malformed);
        assert_eq!(
            parse_shared("small_primes 2 5\nsmall_primes_length 3\n").unwrap_err(),
            Error::Malformed
        );
    }
}
