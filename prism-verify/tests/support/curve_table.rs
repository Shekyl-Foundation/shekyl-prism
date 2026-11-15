// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

// The columns of `p324_curves.tsv`, by name.
//
// The header comment is the schema. A reader asks for a column by the
// name written there. Reordering the columns does not move a value onto
// a different name. A missing name, a repeated name, or a row whose
// width is not the header's, refuses the file.
//
// `//` rather than `//!`: `pairing.rs` includes this file, and an inner
// doc comment is not legal in the middle of that module.

/// Header tokens, in the order the Sage script writes them. The parser
/// matches the set of names, not this order.
const COLUMNS: [&str; 14] = [
    "CURVE",
    "n",
    "A",
    "index",
    "square",
    "x(R)",
    "x([3]R)",
    "x([2]R)",
    "x([2^e]R)",
    "x(P)",
    "y(P)",
    "x(Q)",
    "y(Q)",
    "e(P,Q)",
];

/// One `CURVE` row. The strings are the file's fields, still hex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurveRow<'a> {
    /// Column `n`.
    pub number: &'a str,
    /// Column `A`, the Montgomery coefficient.
    pub coefficient: &'a str,
    /// Column `index`, the basis search.
    pub index: u16,
    /// Column `square`: `1` when `A` is a square, `0` when it is not.
    pub coefficient_is_square: bool,
    /// Column `x(R)`, a point of order `p + 1`.
    pub order_full_x: &'a str,
    /// Column `x([3]R)`, order `(p + 1)/3`.
    pub order_without_three_x: &'a str,
    /// Column `x([2]R)`, order `(p + 1)/2`.
    pub order_without_two_x: &'a str,
    /// Column `x([2^e]R)`, order `3`.
    pub order_three_x: &'a str,
    /// Column `x(P)`.
    pub p_x: &'a str,
    /// Column `y(P)`.
    pub p_y: &'a str,
    /// Column `x(Q)`.
    pub q_x: &'a str,
    /// Column `y(Q)`.
    pub q_y: &'a str,
    /// Column `e(P,Q)`.
    pub pairing: &'a str,
}

/// Every `CURVE` row, in file order.
pub fn parse(table: &str) -> Result<Vec<CurveRow<'_>>, String> {
    let at = header(table)?;
    let mut rows = Vec::new();
    for line in table.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != COLUMNS.len() {
            return Err(format!(
                "a curve row has {} fields and the header has {}",
                fields.len(),
                COLUMNS.len()
            ));
        }
        let take = |name: &str| fields[file_index(&at, name)];
        if take("CURVE") != "CURVE" {
            return Err(format!("a row is tagged {}, not CURVE", take("CURVE")));
        }
        let number = take("n");
        if rows.iter().any(|row: &CurveRow<'_>| row.number == number) {
            return Err(format!("curve {number} is repeated"));
        }
        let coefficient_is_square = match take("square") {
            "1" => true,
            "0" => false,
            other => return Err(format!("curve {number}: square flag {other}")),
        };
        let index = take("index")
            .parse::<u16>()
            .map_err(|_| format!("curve {number}: index {}", take("index")))?;
        rows.push(CurveRow {
            number,
            coefficient: take("A"),
            index,
            coefficient_is_square,
            order_full_x: take("x(R)"),
            order_without_three_x: take("x([3]R)"),
            order_without_two_x: take("x([2]R)"),
            order_three_x: take("x([2^e]R)"),
            p_x: take("x(P)"),
            p_y: take("y(P)"),
            q_x: take("x(Q)"),
            q_y: take("y(Q)"),
            pairing: take("e(P,Q)"),
        });
    }
    if rows.is_empty() {
        return Err("the curve file has no CURVE row".into());
    }
    Ok(rows)
}

/// File index of each column name. `None` unless the tokens are exactly
/// [`COLUMNS`], each once.
fn layout(tokens: &[&str]) -> Option<[usize; COLUMNS.len()]> {
    if tokens.len() != COLUMNS.len() {
        return None;
    }
    let mut at = [0; COLUMNS.len()];
    for (slot, name) in COLUMNS.iter().enumerate() {
        let mut found = None;
        for (index, token) in tokens.iter().enumerate() {
            if token == name {
                if found.is_some() {
                    return None;
                }
                found = Some(index);
            }
        }
        at[slot] = found?;
    }
    Some(at)
}

/// File index of `name`. `name` is one of [`COLUMNS`]; a typo panics
/// here, which is this parser's own bug, not a bad file.
fn file_index(at: &[usize; COLUMNS.len()], name: &str) -> usize {
    let slot = COLUMNS
        .iter()
        .position(|column| *column == name)
        .unwrap_or_else(|| panic!("column {name} is not in the schema"));
    at[slot]
}

fn header(table: &str) -> Result<[usize; COLUMNS.len()], String> {
    let mut found = None;
    for line in table.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        let Some(at) = layout(&tokens) else {
            continue;
        };
        if found.is_some() {
            return Err("the curve file names its columns twice".into());
        }
        found = Some(at);
    }
    found.ok_or_else(|| format!("the curve file does not name {}", COLUMNS.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::{parse, COLUMNS};

    fn values_in_column_order() -> [&'static str; COLUMNS.len()] {
        [
            "CURVE", "7", "coeff", "1", "0", "full", "no3", "no2", "ord3", "px", "py", "qx", "qy",
            "pair",
        ]
    }

    #[test]
    fn a_column_moves_with_its_name() {
        let values = values_in_column_order();
        let header = COLUMNS.iter().rev().copied().collect::<Vec<_>>().join(" ");
        let body = values.iter().rev().copied().collect::<Vec<_>>().join("\t");
        let table = format!("# {header}\n{body}\n");
        let rows = parse(&table).expect("a permuted header");
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row.number, "7");
        assert_eq!(row.coefficient, "coeff");
        assert_eq!(row.index, 1);
        assert!(!row.coefficient_is_square);
        assert_eq!(row.order_full_x, "full");
        assert_eq!(row.order_without_three_x, "no3");
        assert_eq!(row.order_without_two_x, "no2");
        assert_eq!(row.order_three_x, "ord3");
        assert_eq!(row.p_x, "px");
        assert_eq!(row.p_y, "py");
        assert_eq!(row.q_x, "qx");
        assert_eq!(row.q_y, "qy");
        assert_eq!(row.pairing, "pair");
    }

    #[test]
    fn a_missing_or_short_or_repeated_row_is_refused() {
        assert!(
            parse("# CURVE n A\nCURVE\t1\taa\n").is_err(),
            "a short header"
        );
        let header = COLUMNS.join(" ");
        assert!(
            parse(&format!("# {header}\nCURVE\t1\n")).is_err(),
            "a short row"
        );
        let straight = values_in_column_order().join("\t");
        assert!(
            parse(&format!("# {header}\n{straight}\n{straight}\n")).is_err(),
            "curve 7 twice"
        );
        assert!(parse("# not a header\n").is_err(), "no header");
    }

    #[test]
    fn the_committed_file_names_every_column() {
        let rows = parse(include_str!("../../testdata/p324_curves.tsv")).expect("committed curves");
        assert_eq!(rows.len(), 8);
        let numbers: Vec<&str> = rows.iter().map(|row| row.number).collect();
        assert_eq!(numbers, ["1", "2", "3", "4", "5", "6", "7", "8"]);
    }
}
