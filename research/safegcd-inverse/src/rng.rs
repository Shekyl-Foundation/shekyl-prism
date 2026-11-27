//! SplitMix64: a small deterministic generator for tests and benchmarks.

/// SplitMix64 state.
pub struct Rng(pub u64);

impl Rng {
    /// Next 64 bits.
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// `bits` random bits as little-endian limbs (top bit forced when `top`).
    pub fn bits(&mut self, bits: u32, top: bool) -> Vec<u64> {
        let n = bits.div_ceil(64) as usize;
        let mut v: Vec<u64> = (0..n).map(|_| self.next()).collect();
        let r = bits % 64;
        if r != 0 {
            v[n - 1] &= (1u64 << r) - 1;
        }
        if top {
            v[n - 1] |= 1u64 << ((bits - 1) % 64);
        }
        v
    }
}
