// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

// A Sage curve file, read by column name into typed values.
//
// `scripts/sage/p324_fixture.sage` writes one `CURVE` row for each curve,
// under a header comment that names the columns. That header is the
// schema. A reader asks for a column by name, so reordering the columns
// does not move a value onto a different name. A missing name, a repeated
// name, a row whose width is not the header's, a field that is not hex
// where hex is due, or a repeated curve, refuses the file.
//
// This file is shared by inclusion: the tests of `prism-verify` and its
// count reporter take it with `#[path]`, the pairing tests of
// `prism-isogeny` with `include!`, and so does the fuzz crate. It
// therefore uses nothing but `prism_field`, which every one of them has
// under that name, and its comments are `//` because an inner doc comment
// is not legal in the middle of the module that includes it. No includer
// uses every item, so each allows dead code on its `mod` line.

use prism_field::ParameterSet;

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
    OrderWithoutThree,
    OrderWithoutTwo,
    OrderThree,
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
        Self::OrderWithoutThree,
        Self::OrderWithoutTwo,
        Self::OrderThree,
        Self::PX,
        Self::PY,
        Self::QX,
        Self::QY,
        Self::Pairing,
    ];

    /// The name the header gives this column.
    const fn name(self) -> &'static str {
        match self {
            Self::Tag => ROW_TAG,
            Self::Number => "n",
            Self::Coefficient => "A",
            Self::Index => "index",
            Self::Square => "square",
            Self::OrderFull => "x(R)",
            Self::OrderWithoutThree => "x([3]R)",
            Self::OrderWithoutTwo => "x([2]R)",
            Self::OrderThree => "x([2^e]R)",
            Self::PX => "x(P)",
            Self::PY => "y(P)",
            Self::QX => "x(Q)",
            Self::QY => "y(Q)",
            Self::Pairing => "e(P,Q)",
        }
    }
}

/// Where each column is in one file: the position of its name in that
/// file's header.
struct Layout([usize; Column::ALL.len()]);

impl Layout {
    /// `None` unless `names` is exactly the column names, each once.
    fn of(names: &[&str]) -> Option<Self> {
        if names.len() != Column::ALL.len() {
            return None;
        }
        let mut positions = [0; Column::ALL.len()];
        for (slot, column) in positions.iter_mut().zip(Column::ALL) {
            let mut matches = names
                .iter()
                .enumerate()
                .filter(|(_, name)| **name == column.name());
            let (position, _) = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            *slot = position;
        }
        Some(Self(positions))
    }

    /// The one header comment of `table` that names the columns.
    fn read(table: &str) -> Result<Self, String> {
        let mut layouts = table
            .lines()
            .filter_map(|line| line.strip_prefix('#'))
            .filter_map(|rest| Self::of(&rest.split_whitespace().collect::<Vec<_>>()));
        let layout = layouts.next().ok_or_else(|| {
            let names: Vec<&str> = Column::ALL.into_iter().map(Column::name).collect();
            format!("the curve file does not name {}", names.join(" "))
        })?;
        if layouts.next().is_some() {
            return Err("the curve file names its columns twice".into());
        }
        Ok(layout)
    }

    /// The field of `column` in a row already known to have every column.
    fn field<'a>(&self, fields: &[&'a str], column: Column) -> &'a str {
        fields[self.0[column as usize]]
    }
}

/// The number a curve has in its file, column `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurveNumber(u8);

impl core::fmt::Display for CurveNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

/// The bytes a string of hex digits spells, in either case. `None` for
/// half a byte or for anything that is not a hex digit.
pub fn hex(text: &str) -> Option<Vec<u8>> {
    let nibble = |digit: u8| char::from(digit).to_digit(16);
    let byte = |pair: &[u8]| u8::try_from(nibble(pair[0])? << 4 | nibble(pair[1])?).ok();
    let pairs = text.as_bytes().chunks_exact(2);
    if !pairs.remainder().is_empty() {
        return None;
    }
    pairs.map(byte).collect()
}

/// One element of `F_p²` as the file has it: hex, real part first.
///
/// It is hex of whole bytes by the time it exists, since the parser
/// refuses anything else. Whether it is below `p` is known only at a
/// parameter set, so [`Element::decode`] is where that is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Element<'a>(&'a str);

impl<'a> Element<'a> {
    fn new(text: &'a str) -> Option<Self> {
        hex(text).map(|_| Self(text))
    }

    /// The bytes the hex spells, which for a coefficient is a public key.
    pub fn bytes(self) -> Vec<u8> {
        hex(self.0).expect("checked to be hex when it was read")
    }

    /// The element at the set `L`. The file is committed and written by
    /// Sage, so a value that is not canonical there is a broken fixture.
    pub fn decode<L: ParameterSet>(self) -> L::Ext {
        L::ext_decode(&self.bytes()).expect("a committed element is canonical")
    }
}

/// A point as the file has it: two elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point<'a> {
    pub x: Element<'a>,
    pub y: Element<'a>,
}

impl Point<'_> {
    /// `(x, y)` at the set `L`.
    pub fn decode<L: ParameterSet>(self) -> (L::Ext, L::Ext) {
        (self.x.decode::<L>(), self.y.decode::<L>())
    }
}

/// One `CURVE` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurveRow<'a> {
    /// Column `n`.
    pub number: CurveNumber,
    /// Column `A`, the Montgomery coefficient.
    pub coefficient: Element<'a>,
    /// Column `index`, the one the basis search names.
    pub index: u16,
    /// Column `square`: whether `A` is a square, which is the branch the
    /// basis search takes.
    pub coefficient_is_square: bool,
    /// Column `x(R)`, a point of order `p + 1`.
    pub order_full_x: Element<'a>,
    /// Column `x([3]R)`, order `(p + 1)/3`.
    pub order_without_three_x: Element<'a>,
    /// Column `x([2]R)`, order `(p + 1)/2`.
    pub order_without_two_x: Element<'a>,
    /// Column `x([2^e]R)`, order `3`.
    pub order_three_x: Element<'a>,
    /// Columns `x(P)` and `y(P)`.
    pub p: Point<'a>,
    /// Columns `x(Q)` and `y(Q)`. This `Q` is the second point the basis
    /// search names. The verifier's canonical second point is the
    /// difference of the two, so up to sign this is `P − Q′` for that `Q′`.
    pub q: Point<'a>,
    /// Column `e(P,Q)`.
    pub pairing: Element<'a>,
}

/// Every `CURVE` row of one file, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurveTable<'a>(Vec<CurveRow<'a>>);

impl<'a> CurveTable<'a> {
    pub fn parse(table: &'a str) -> Result<Self, String> {
        let layout = Layout::read(table)?;
        let mut rows: Vec<CurveRow<'a>> = Vec::new();
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
                    column.name(),
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
                order_without_three_x: element(Column::OrderWithoutThree)?,
                order_without_two_x: element(Column::OrderWithoutTwo)?,
                order_three_x: element(Column::OrderThree)?,
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

    pub fn rows(&self) -> &[CurveRow<'a>] {
        &self.0
    }

    /// The row of one curve. A number the file does not have is a mistake
    /// in the caller or a changed fixture, and both stop the test.
    pub fn curve(&self, number: CurveNumber) -> CurveRow<'a> {
        *self
            .0
            .iter()
            .find(|row| row.number == number)
            .unwrap_or_else(|| panic!("the curve file has no curve {number}"))
    }
}

/// The curve file at `p324`, and the curves of it that tests name.
///
/// A curve is named here by what it is used for. What makes it fit for
/// that use is a property of its coefficient, held by a test and not by
/// this constant: `p324_curves` holds the branch of each, and
/// `p324_refusals` holds which coefficients the model check passes.
pub mod p324 {
    use super::{CurveNumber, CurveTable};

    /// The file's name, as the count records cite it.
    pub const FILE: &str = "p324_curves.tsv";

    const TABLE: &str = include_str!("../../testdata/p324_curves.tsv");

    /// How many curves the Sage script walks to.
    pub const CURVES: usize = 8;

    /// A curve whose coefficient is a square, so the basis search takes
    /// its square branch.
    pub const SQUARE_BRANCH_CURVE: CurveNumber = CurveNumber(1);

    /// A curve off the square branch whose coefficient is the permitted
    /// model of its class, so the verifier takes it as a response curve.
    pub const MODEL_CURVE: CurveNumber = CurveNumber(7);

    /// A curve whose negated coefficient is the permitted model of its
    /// class. With [`MODEL_CURVE`] it is all the file has of those.
    pub const NEGATED_MODEL_CURVE: CurveNumber = CurveNumber(5);

    /// One curve from each branch of the basis search, with whether its
    /// coefficient is a square.
    pub const BRANCH_CURVES: [(CurveNumber, bool); 2] =
        [(SQUARE_BRANCH_CURVE, true), (MODEL_CURVE, false)];

    /// The committed file. It is the Sage script's output, so a file this
    /// does not read, or one with another number of curves, is a broken
    /// fixture.
    pub fn table() -> CurveTable<'static> {
        let table = CurveTable::parse(TABLE).unwrap_or_else(|reason| panic!("{FILE}: {reason}"));
        assert_eq!(table.rows().len(), CURVES, "{FILE}");
        table
    }
}

#[cfg(test)]
mod tests {
    use super::{hex, p324, Column, CurveNumber, CurveTable, Element, Layout};

    /// One row's fields in the order of [`Column::ALL`], with a different
    /// value in every hex column.
    const FIELDS: [&str; Column::ALL.len()] = [
        "CURVE", "7", "a0", "1", "0", "f1", "03", "02", "33", "b1", "b2", "c1", "c2", "ee",
    ];

    fn header() -> String {
        Column::ALL.map(Column::name).join(" ")
    }

    #[test]
    fn a_column_is_its_own_position_in_the_list() {
        for (position, column) in Column::ALL.into_iter().enumerate() {
            assert_eq!(column as usize, position, "{}", column.name());
        }
    }

    #[test]
    fn a_column_moves_with_its_name() {
        let names: Vec<&str> = Column::ALL.into_iter().rev().map(Column::name).collect();
        let fields: Vec<&str> = FIELDS.into_iter().rev().collect();
        let text = format!("# {}\n{}\n", names.join(" "), fields.join("\t"));
        let table = CurveTable::parse(&text).expect("a permuted header");
        assert_eq!(table.rows().len(), 1);
        let row = table.curve(CurveNumber(7));
        let hex = |element: Element<'_>| element.bytes();
        assert_eq!(hex(row.coefficient), [0xa0]);
        assert_eq!(row.index, 1);
        assert!(!row.coefficient_is_square);
        assert_eq!(hex(row.order_full_x), [0xf1]);
        assert_eq!(hex(row.order_without_three_x), [0x03]);
        assert_eq!(hex(row.order_without_two_x), [0x02]);
        assert_eq!(hex(row.order_three_x), [0x33]);
        assert_eq!((hex(row.p.x), hex(row.p.y)), (vec![0xb1], vec![0xb2]));
        assert_eq!((hex(row.q.x), hex(row.q.y)), (vec![0xc1], vec![0xc2]));
        assert_eq!(hex(row.pairing), [0xee]);
    }

    #[test]
    fn hex_is_read_in_either_case_and_only_in_whole_bytes() {
        assert_eq!(hex("00ff10aB"), Some(vec![0x00, 0xff, 0x10, 0xab]));
        assert_eq!(hex(""), Some(Vec::new()));
        assert_eq!(hex("abc"), None, "half a byte");
        assert_eq!(hex("0g"), None, "not a hex digit");
        assert_eq!(hex("+1"), None, "a sign is not a hex digit");
        assert_eq!(Element::new("0a").map(Element::bytes), Some(vec![0x0a]));
        assert_eq!(Element::new("0"), None);
    }

    #[test]
    fn a_file_that_is_not_the_schema_is_refused() {
        let header = header();
        let row = FIELDS.join("\t");
        let parsed = |text: &str| CurveTable::parse(text).map(|table| table.rows().len());
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
            fields[column as usize] = value;
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
    fn the_committed_file_is_read_and_has_the_named_curves() {
        let table = p324::table();
        let numbers: Vec<String> = table
            .rows()
            .iter()
            .map(|row| row.number.to_string())
            .collect();
        assert_eq!(numbers, ["1", "2", "3", "4", "5", "6", "7", "8"]);
        for (number, square) in p324::BRANCH_CURVES {
            assert_eq!(
                table.curve(number).coefficient_is_square,
                square,
                "{number}"
            );
        }
        assert_eq!(
            table.curve(p324::NEGATED_MODEL_CURVE).number.to_string(),
            "5"
        );
        // The schema is the header the committed file has.
        Layout::read(&format!("# {}\n", header())).expect("the schema names itself");
    }
}
