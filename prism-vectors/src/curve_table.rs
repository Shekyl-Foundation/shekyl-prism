// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! A Sage curve file, read by column name into typed values.
//!
//! `scripts/sage/no_corpus_fixture.sage` writes one `CURVE` row for each
//! curve, under a header comment that names the columns. That header is
//! the schema. A reader asks for a column by name, so reordering the columns
//! does not move a value onto a different name. A missing name, a repeated
//! name, a row whose width is not the header's, a field that is not hex
//! where hex is due, or a repeated curve, refuses the file.
//!
//! Nothing here is particular to one prime but one column's name: the
//! point missing the cofactor's odd prime is `x([3]R)` in the file of a
//! prime with `f = 3` and `x([17]R)` where `f = 17`. So a file is read
//! against a cofactor, and a header written for another cofactor is not
//! this schema. A cofactor with two odd primes would need two such
//! columns; no set that has a curve file has one, and the reader refuses
//! it.

use crate::hex;
use prism_field::{Cofactor, ParameterSet};

/// The first field of every data row.
const ROW_TAG: &str = "CURVE";

/// The columns, in the order the Sage script writes them. The parser
/// matches the set of names in a file's header, not this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    Tag,
    Number,
    Coefficient,
    Index,
    Square,
    OrderFull,
    OrderWithoutOddPrime,
    OrderWithoutTwo,
    OrderOddPrime,
    PX,
    PY,
    QX,
    QY,
    Pairing,
}

impl Column {
    const ALL: [Self; 14] = [
        Self::Tag,
        Self::Number,
        Self::Coefficient,
        Self::Index,
        Self::Square,
        Self::OrderFull,
        Self::OrderWithoutOddPrime,
        Self::OrderWithoutTwo,
        Self::OrderOddPrime,
        Self::PX,
        Self::PY,
        Self::QX,
        Self::QY,
        Self::Pairing,
    ];

    /// The name the header gives this column, in the file of a set whose
    /// cofactor has the one odd prime `odd_prime`.
    fn name(self, odd_prime: u64) -> String {
        match self {
            Self::Tag => ROW_TAG.into(),
            Self::Number => "n".into(),
            Self::Coefficient => "A".into(),
            Self::Index => "index".into(),
            Self::Square => "square".into(),
            Self::OrderFull => "x(R)".into(),
            Self::OrderWithoutOddPrime => format!("x([{odd_prime}]R)"),
            Self::OrderWithoutTwo => "x([2]R)".into(),
            Self::OrderOddPrime => "x([2^e]R)".into(),
            Self::PX => "x(P)".into(),
            Self::PY => "y(P)".into(),
            Self::QX => "x(Q)".into(),
            Self::QY => "y(Q)".into(),
            Self::Pairing => "e(P,Q)".into(),
        }
    }

    /// The slot of this column in a [`Layout`].
    ///
    /// The number is this match, not the enum discriminant and not the
    /// position of the variant in [`Self::ALL`]. Reordering either of
    /// those leaves a parsed row on the same column.
    const fn index(self) -> usize {
        match self {
            Self::Tag => 0,
            Self::Number => 1,
            Self::Coefficient => 2,
            Self::Index => 3,
            Self::Square => 4,
            Self::OrderFull => 5,
            Self::OrderWithoutOddPrime => 6,
            Self::OrderWithoutTwo => 7,
            Self::OrderOddPrime => 8,
            Self::PX => 9,
            Self::PY => 10,
            Self::QX => 11,
            Self::QY => 12,
            Self::Pairing => 13,
        }
    }
}

/// Where each column is in one file: the position of its name in that
/// file's header.
struct Layout {
    positions: [usize; Column::ALL.len()],
    /// The odd prime the header was read for, to name a column in an
    /// error.
    odd_prime: u64,
}

/// The one odd prime of `cofactor`, which names a column of its file.
fn odd_prime(cofactor: Cofactor) -> Result<u64, String> {
    match cofactor.odd_primes() {
        [prime] => Ok(*prime),
        primes => Err(format!(
            "a curve file has one column for the cofactor's odd prime, and {} has {}",
            cofactor.value(),
            primes.len()
        )),
    }
}

impl Layout {
    /// `None` unless `names` is exactly the column names, each once.
    fn of(names: &[&str], odd_prime: u64) -> Option<Self> {
        if names.len() != Column::ALL.len() {
            return None;
        }
        let mut positions = [0; Column::ALL.len()];
        for column in Column::ALL {
            let mut matches = names
                .iter()
                .enumerate()
                .filter(|(_, name)| **name == column.name(odd_prime));
            let (position, _) = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            positions[column.index()] = position;
        }
        Some(Self {
            positions,
            odd_prime,
        })
    }

    /// The one header comment of `table` that names the columns of a
    /// file for `cofactor`.
    fn read(table: &str, cofactor: Cofactor) -> Result<Self, String> {
        let odd_prime = odd_prime(cofactor)?;
        let mut layouts = table
            .lines()
            .filter_map(|line| line.strip_prefix('#'))
            .filter_map(|rest| Self::of(&rest.split_whitespace().collect::<Vec<_>>(), odd_prime));
        let layout = layouts.next().ok_or_else(|| {
            let names: Vec<String> = Column::ALL
                .into_iter()
                .map(|column| column.name(odd_prime))
                .collect();
            format!("the curve file does not name {}", names.join(" "))
        })?;
        if layouts.next().is_some() {
            return Err("the curve file names its columns twice".into());
        }
        Ok(layout)
    }

    /// The field of `column` in a row already known to have every column.
    fn field<'a>(&self, fields: &[&'a str], column: Column) -> &'a str {
        fields[self.positions[column.index()]]
    }
}

/// The number a curve has in its file, column `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurveNumber(u8);

impl CurveNumber {
    /// The curve the file numbers `number`.
    pub const fn new(number: u8) -> Self {
        Self(number)
    }
}

impl core::fmt::Display for CurveNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

/// One element of `F_p²`: the bytes its hex spelled, real part first.
///
/// The text is decoded once, when the row is read, and this holds those
/// bytes. Whether they are below `p` is known only at a parameter set, so
/// [`Element::decode`] is where that is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element(Vec<u8>);

impl Element {
    fn new(text: &str) -> Option<Self> {
        hex::decode(text).map(Self)
    }

    /// The bytes the hex spelled, which for a coefficient is a public key.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    /// The element at the set `L`. The file is committed and written by
    /// Sage, so a value that is not canonical there is a broken fixture.
    pub fn decode<L: ParameterSet>(&self) -> L::Ext {
        L::ext_decode(self.bytes()).expect("a committed element is canonical")
    }
}

/// A point as the file has it: two elements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    /// The abscissa.
    pub x: Element,
    /// The ordinate.
    pub y: Element,
}

impl Point {
    /// `(x, y)` at the set `L`.
    pub fn decode<L: ParameterSet>(&self) -> (L::Ext, L::Ext) {
        (self.x.decode::<L>(), self.y.decode::<L>())
    }
}

/// One `CURVE` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurveRow {
    /// Column `n`.
    pub number: CurveNumber,
    /// Column `A`, the Montgomery coefficient.
    pub coefficient: Element,
    /// Column `index`, the one the basis search names.
    pub index: u16,
    /// Column `square`: whether `A` is a square, which is the branch the
    /// basis search takes.
    pub coefficient_is_square: bool,
    /// Column `x(R)`, a point of order `p + 1`.
    pub order_full_x: Element,
    /// Column `x([q]R)` for the cofactor's odd prime `q`, order `(p + 1)/q`.
    pub order_without_odd_prime_x: Element,
    /// Column `x([2]R)`, order `(p + 1)/2`.
    pub order_without_two_x: Element,
    /// Column `x([2^e]R)`, of order the cofactor.
    pub order_odd_prime_x: Element,
    /// Columns `x(P)` and `y(P)`.
    pub p: Point,
    /// Columns `x(Q)` and `y(Q)`. This `Q` is the second point the basis
    /// search names. The verifier's canonical second point is the
    /// difference of the two, so up to sign this is `P − Q′` for that `Q′`.
    pub q: Point,
    /// Column `e(P,Q)`.
    pub pairing: Element,
}

/// Every `CURVE` row of one file, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurveTable(Vec<CurveRow>);

impl CurveTable {
    /// The rows of `table`, the file of a set with this `cofactor`.
    pub fn parse(table: &str, cofactor: Cofactor) -> Result<Self, String> {
        let layout = Layout::read(table, cofactor)?;
        let mut rows: Vec<CurveRow> = Vec::new();
        for line in table.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != Column::ALL.len() {
                return Err(format!(
                    "a row has {} fields and the header names {}",
                    fields.len(),
                    Column::ALL.len()
                ));
            }
            let text = |column| layout.field(&fields, column);
            if text(Column::Tag) != ROW_TAG {
                return Err(format!(
                    "a row is tagged {}, not {ROW_TAG}",
                    text(Column::Tag)
                ));
            }
            let refused = |column: Column| {
                format!(
                    "curve {}: {} is {}",
                    text(Column::Number),
                    column.name(layout.odd_prime),
                    text(column)
                )
            };
            let element = |column| Element::new(text(column)).ok_or_else(|| refused(column));
            let number = text(Column::Number)
                .parse()
                .map(CurveNumber)
                .map_err(|_| refused(Column::Number))?;
            if rows.iter().any(|row| row.number == number) {
                return Err(format!("curve {number} is repeated"));
            }
            rows.push(CurveRow {
                number,
                coefficient: element(Column::Coefficient)?,
                index: text(Column::Index)
                    .parse()
                    .map_err(|_| refused(Column::Index))?,
                coefficient_is_square: match text(Column::Square) {
                    "1" => true,
                    "0" => false,
                    _ => return Err(refused(Column::Square)),
                },
                order_full_x: element(Column::OrderFull)?,
                order_without_odd_prime_x: element(Column::OrderWithoutOddPrime)?,
                order_without_two_x: element(Column::OrderWithoutTwo)?,
                order_odd_prime_x: element(Column::OrderOddPrime)?,
                p: Point {
                    x: element(Column::PX)?,
                    y: element(Column::PY)?,
                },
                q: Point {
                    x: element(Column::QX)?,
                    y: element(Column::QY)?,
                },
                pairing: element(Column::Pairing)?,
            });
        }
        if rows.is_empty() {
            return Err(format!("the curve file has no {ROW_TAG} row"));
        }
        Ok(Self(rows))
    }

    /// Every curve of the file, in file order.
    pub fn rows(&self) -> &[CurveRow] {
        &self.0
    }

    /// The row of one curve. A number the file does not have is a mistake
    /// in the caller or a changed fixture, and both stop the test.
    ///
    /// The row is copied so the caller need not keep the table.
    pub fn curve(&self, number: CurveNumber) -> CurveRow {
        self.0
            .iter()
            .find(|row| row.number == number)
            .unwrap_or_else(|| panic!("the curve file has no curve {number}"))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{Column, CurveNumber, CurveTable, Element, Layout};
    use prism_field::Cofactor;

    /// The cofactor the test files below are written for.
    const ODD_PRIME: u64 = 3;

    fn cofactor(value: u64, odd_primes: &'static [u64]) -> Cofactor {
        Cofactor::new(value, odd_primes).expect("a cofactor")
    }

    fn three() -> Cofactor {
        cofactor(ODD_PRIME, &[ODD_PRIME])
    }

    fn names() -> Vec<String> {
        Column::ALL
            .into_iter()
            .map(|column| column.name(ODD_PRIME))
            .collect()
    }

    /// One row's fields in the order of [`Column::ALL`], with a different
    /// value in every hex column.
    const FIELDS: [&str; Column::ALL.len()] = [
        "CURVE", "7", "a0", "1", "0", "f1", "03", "02", "33", "b1", "b2", "c1", "c2", "ee",
    ];

    fn header() -> String {
        names().join(" ")
    }

    /// Where `column` sits in [`Column::ALL`], which is the order [`FIELDS`]
    /// and [`header`] are written in. Distinct from [`Column::index`].
    fn listed_at(column: Column) -> usize {
        Column::ALL
            .iter()
            .position(|item| *item == column)
            .expect("every column is in the list")
    }

    #[test]
    fn every_column_has_its_own_slot() {
        let mut seen = [false; Column::ALL.len()];
        for column in Column::ALL {
            let slot = column.index();
            assert!(slot < seen.len(), "{}", column.name(ODD_PRIME));
            assert!(!seen[slot], "{} shares a slot", column.name(ODD_PRIME));
            seen[slot] = true;
        }
    }

    #[test]
    fn a_column_moves_with_its_name() {
        let names: Vec<String> = names().into_iter().rev().collect();
        let fields: Vec<&str> = FIELDS.into_iter().rev().collect();
        let text = format!("# {}\n{}\n", names.join(" "), fields.join("\t"));
        let table = CurveTable::parse(&text, three()).expect("a permuted header");
        assert_eq!(table.rows().len(), 1);
        let row = table.curve(CurveNumber::new(7));
        assert_eq!(row.coefficient.bytes(), &[0xa0]);
        assert_eq!(row.index, 1);
        assert!(!row.coefficient_is_square);
        assert_eq!(row.order_full_x.bytes(), &[0xf1]);
        assert_eq!(row.order_without_odd_prime_x.bytes(), &[0x03]);
        assert_eq!(row.order_without_two_x.bytes(), &[0x02]);
        assert_eq!(row.order_odd_prime_x.bytes(), &[0x33]);
        assert_eq!(
            (row.p.x.bytes(), row.p.y.bytes()),
            (&[0xb1][..], &[0xb2][..])
        );
        assert_eq!(
            (row.q.x.bytes(), row.q.y.bytes()),
            (&[0xc1][..], &[0xc2][..])
        );
        assert_eq!(row.pairing.bytes(), &[0xee]);
    }

    #[test]
    fn an_element_is_whole_bytes_of_hex() {
        assert_eq!(Element::new("0a").expect("hex").bytes(), &[0x0a]);
        assert_eq!(Element::new("0"), None, "half a byte");
        assert_eq!(Element::new("zz"), None, "not hex");
    }

    #[test]
    fn a_file_that_is_not_the_schema_is_refused() {
        let header = header();
        let row = FIELDS.join("\t");
        let parsed = |text: &str| CurveTable::parse(text, three()).map(|table| table.rows().len());
        assert_eq!(parsed(&format!("# {header}\n{row}\n")), Ok(1));
        let refused = |text: String, why: &str| assert!(parsed(&text).is_err(), "{why}");
        refused("# not a header\n".into(), "no header");
        refused(format!("# CURVE n A\n{row}\n"), "a short header");
        refused(
            format!("# {}\n{row}\n", header.replace("y(Q)", "y(P)")),
            "a name twice and a name missing",
        );
        refused(format!("# {header}\n# {header}\n{row}\n"), "two headers");
        refused(format!("# {header}\n"), "no row");
        refused(format!("# {header}\nCURVE\t1\n"), "a short row");
        refused(format!("# {header}\n{row}\n{row}\n"), "a curve twice");
        let with = |column: Column, value: &str| {
            let mut fields = FIELDS;
            fields[listed_at(column)] = value;
            format!("# {header}\n{}\n", fields.join("\t"))
        };
        refused(with(Column::Tag, "POINT"), "another tag");
        refused(with(Column::Number, "seven"), "a number that is not one");
        refused(with(Column::Index, "-1"), "an index that is not one");
        refused(with(Column::Square, "2"), "a square flag that is not one");
        refused(with(Column::Coefficient, "a"), "half a byte");
        refused(with(Column::PY, "zz"), "not hex");
    }

    #[test]
    fn the_schema_names_itself() {
        Layout::read(&format!("# {}\n", header()), three()).expect("a header of every column name");
    }

    /// One column is named for the cofactor's odd prime, so a file is the
    /// schema of its own cofactor and of no other.
    #[test]
    fn a_file_is_read_against_its_own_cofactor() {
        let text = format!("# {}\n{}\n", header(), FIELDS.join("\t"));
        let seventeen = cofactor(17, &[17]);
        assert!(CurveTable::parse(&text, three()).is_ok());
        assert!(
            CurveTable::parse(&text, seventeen).is_err(),
            "a header for f = 3 is not the schema at f = 17"
        );
        let for_seventeen = text.replace("x([3]R)", "x([17]R)");
        assert!(CurveTable::parse(&for_seventeen, seventeen).is_ok());
        assert!(CurveTable::parse(&for_seventeen, three()).is_err());
        // Two odd primes would need two such columns.
        let two_primes = cofactor(65, &[5, 13]);
        let reason = CurveTable::parse(&text, two_primes).expect_err("no such file");
        assert!(reason.contains("65 has 2"), "{reason}");
        // A cofactor that is a power of its prime has the one column.
        let power = cofactor(27, &[3]);
        assert!(CurveTable::parse(&text, power).is_ok());
    }
}
