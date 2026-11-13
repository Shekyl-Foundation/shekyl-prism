// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Counts of field operations, the machine-independent cost of a
//! verification.
//!
//! The functions here always exist. Without the `op-count` feature
//! [`tally`] and [`enter`] have empty bodies and [`take`] returns zeros, so
//! a caller marks its stages unconditionally and a default build carries no
//! counter. With the feature, each thread keeps its own table: one row per
//! slot, and the caller says which slot is current with [`enter`].
//!
//! What is counted is a call through the [`Field`](crate::Field) trait.
//! An extension-field multiplication is one `Ext` multiplication; the
//! base-field multiplications `fp2` performs inside it are not counted
//! again. A `Base` count is a base-field call the caller made itself.
//! Additions, subtractions, negations and comparisons are not counted.

/// Slots a caller may [`enter`]. Slot 0 is current after [`take`].
pub const SLOTS: usize = 16;

/// Whether this build counts. `false` means every count reads zero.
pub const ENABLED: bool = cfg!(feature = "op-count");

/// Which field an operation was in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Base,
    Ext,
}

/// A counted operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Mul,
    Inv,
    Sqrt,
    IsSquare,
}

/// Operations of one field in one slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub mul: u64,
    pub inv: u64,
    pub sqrt: u64,
    pub is_square: u64,
}

impl Counts {
    /// The sum of two counts, field by field.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        Self {
            mul: self.mul + other.mul,
            inv: self.inv + other.inv,
            sqrt: self.sqrt + other.sqrt,
            is_square: self.is_square + other.is_square,
        }
    }
}

/// One slot's counts in both fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slot {
    pub base: Counts,
    pub ext: Counts,
}

#[cfg(feature = "op-count")]
mod table {
    use super::{Kind, Op, Slot, SLOTS};
    use core::cell::Cell;

    thread_local! {
        static CURRENT: Cell<usize> = const { Cell::new(0) };
        static TABLE: Cell<[Slot; SLOTS]> = const {
            Cell::new([Slot {
                base: super::Counts { mul: 0, inv: 0, sqrt: 0, is_square: 0 },
                ext: super::Counts { mul: 0, inv: 0, sqrt: 0, is_square: 0 },
            }; SLOTS])
        };
    }

    pub fn tally(kind: Kind, op: Op) {
        let slot = CURRENT.with(Cell::get);
        TABLE.with(|table| {
            let mut rows = table.get();
            let counts = match kind {
                Kind::Base => &mut rows[slot].base,
                Kind::Ext => &mut rows[slot].ext,
            };
            match op {
                Op::Mul => counts.mul += 1,
                Op::Inv => counts.inv += 1,
                Op::Sqrt => counts.sqrt += 1,
                Op::IsSquare => counts.is_square += 1,
            }
            table.set(rows);
        });
    }

    pub fn enter(slot: usize) {
        assert!(slot < SLOTS, "an op-count slot is below SLOTS");
        CURRENT.with(|current| current.set(slot));
    }

    pub fn take() -> [Slot; SLOTS] {
        CURRENT.with(|current| current.set(0));
        TABLE.with(|table| table.replace([Slot::default(); SLOTS]))
    }
}

/// Count one operation in the current slot.
#[inline(always)]
pub fn tally(kind: Kind, op: Op) {
    #[cfg(feature = "op-count")]
    table::tally(kind, op);
    #[cfg(not(feature = "op-count"))]
    let _ = (kind, op);
}

/// Make `slot` current for this thread. Panics at `SLOTS` or above when
/// counting is on.
#[inline(always)]
pub fn enter(slot: usize) {
    #[cfg(feature = "op-count")]
    table::enter(slot);
    #[cfg(not(feature = "op-count"))]
    let _ = slot;
}

/// This thread's table, which is then cleared with slot 0 current.
#[must_use]
pub fn take() -> [Slot; SLOTS] {
    #[cfg(feature = "op-count")]
    return table::take();
    #[cfg(not(feature = "op-count"))]
    [Slot::default(); SLOTS]
}

#[cfg(all(test, feature = "op-count"))]
mod tests {
    use super::*;
    use crate::{Field, Level, Level1};

    #[test]
    fn each_operation_lands_in_the_current_slot_of_its_field() {
        type Ext = <Level1 as Level>::Ext;
        type Base = <Level1 as Level>::Base;
        let _ = take();
        let x = Ext::from_u64(3);
        let b = Base::from_u64(5);
        enter(2);
        let _ = x.mul(x).mul(x);
        let _ = x.inv();
        let _ = b.mul(b);
        enter(5);
        let _ = x.sqrt();
        let _ = x.is_square();
        let _ = b.is_square();
        // Not counted: additions, comparisons, an inversion of zero is.
        let _ = x.add(x).same(x);
        let _ = Ext::zero().inv();
        let table = take();
        assert_eq!(
            table[2].ext,
            Counts {
                mul: 2,
                inv: 1,
                sqrt: 0,
                is_square: 0
            }
        );
        assert_eq!(
            table[2].base,
            Counts {
                mul: 1,
                inv: 0,
                sqrt: 0,
                is_square: 0
            }
        );
        assert_eq!(
            table[5].ext,
            Counts {
                mul: 0,
                inv: 1,
                sqrt: 1,
                is_square: 1
            }
        );
        assert_eq!(
            table[5].base,
            Counts {
                mul: 0,
                inv: 0,
                sqrt: 0,
                is_square: 1
            }
        );
        assert_eq!(take(), [Slot::default(); SLOTS]);
    }
}
