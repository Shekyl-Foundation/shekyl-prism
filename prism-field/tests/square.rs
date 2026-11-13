// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! `square` is `mul` by itself, in both fields at every level.

use prism_field::{Field, Level, Level1, Level3, Level5};

/// xorshift64*: a fixed stream, so a failure names its sample.
struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A base-field element from `FP_LEN` stream bytes with the top byte
    /// cleared. `2^(8·(FP_LEN−1))` is below `p` at every level, so the
    /// bytes are canonical, and the values cover all but the top of the
    /// range; the named elements below cover the top.
    fn base<L: Level>(&mut self) -> L::Base {
        let mut bytes = vec![0u8; L::FP_LEN];
        for chunk in bytes.chunks_mut(8) {
            let word = self.next().to_le_bytes();
            chunk.copy_from_slice(&word[..chunk.len()]);
        }
        bytes[L::FP_LEN - 1] = 0;
        L::base_decode(&bytes).expect("a value below 2^(8(len-1)) is canonical")
    }

    fn ext<L: Level>(&mut self) -> L::Ext {
        let re = L::ext_from_base(self.base::<L>());
        let im = L::ext_from_base(self.base::<L>());
        re.add(im.mul(L::ext_i()))
    }
}

fn agree<F: Field>(x: F, what: &str) {
    assert!(x.square().same(x.mul(x)), "{what}");
}

fn level<L: Level>(seed: u64) {
    let one = L::Ext::one();
    let minus_one = one.negate();
    let i = L::ext_i();
    // `−1 − i` is the largest canonical element: both halves are `p − 1`.
    for (name, x) in [
        ("zero", L::Ext::zero()),
        ("one", one),
        ("minus one", minus_one),
        ("i", i),
        ("minus i", i.negate()),
        ("one plus i", one.add(i)),
        ("largest canonical", minus_one.sub(i)),
    ] {
        agree(x, &format!("{} extension {name}", L::NAME));
    }
    let base_one = L::Base::one();
    for (name, x) in [
        ("zero", L::Base::zero()),
        ("one", base_one),
        ("largest canonical", base_one.negate()),
        ("two", base_one.add(base_one)),
    ] {
        agree(x, &format!("{} base {name}", L::NAME));
    }
    let mut stream = Stream(seed);
    for n in 0..100_000 {
        agree(
            stream.ext::<L>(),
            &format!("{} extension sample {n}", L::NAME),
        );
        agree(stream.base::<L>(), &format!("{} base sample {n}", L::NAME));
    }
}

#[test]
fn square_equals_mul_by_itself() {
    level::<Level1>(0x5348_454b_594c_0001);
    level::<Level3>(0x5348_454b_594c_0003);
    level::<Level5>(0x5348_454b_594c_0005);
}

/// The comparison can fail: a squaring that dropped the cross term would
/// agree on the base field embedded in the extension and nowhere else.
#[test]
fn the_comparison_distinguishes_a_wrong_square() {
    let x = <Level1 as Level>::Ext::one().add(Level1::ext_i());
    let (re, im) = Level1::ext_parts(x);
    let wrong = Level1::ext_from_base(re.square().sub(im.square()));
    assert!(!wrong.same(x.mul(x)));
}
