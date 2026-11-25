// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! A value barrier for constant-time code, and nothing else.
//!
//! This crate is the proposed *single* place in shekyl-prism where
//! `unsafe` is allowed. It holds exactly one `unsafe` block per target
//! architecture: an empty inline-asm statement that names one register.
//! Every other crate keeps `#![forbid(unsafe_code)]`.
//!
//! Why it exists: LLVM may prove that a mask is 0 or all-ones and turn a
//! branch-free mask select back into a conditional jump. `core::hint::
//! black_box` is documented as best effort only ("must not be relied upon
//! to control critical program behavior"). An empty `asm!` whose output
//! is the input is opaque to every LLVM pass, so the mask's value range
//! is unknown after it. See AUDIT.md for the audit notes.
//!
//! What it does not do: it does not make code constant time. It only
//! stops value-range reasoning at one point. Constant-time behaviour is
//! still checked on the compiled artifact (assembly scan, control-flow
//! trace equivalence, Valgrind taint, dudect).

#![no_std]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!(
    "prism-ct-barrier: no audited barrier for this target_arch; add one (and its audit note) \
     rather than falling back to a best-effort barrier"
);

/// Returns `x` unchanged, through an empty inline-asm statement the
/// optimiser cannot see through.
#[inline(always)]
#[must_use]
pub fn value_barrier_u64(mut x: u64) -> u64 {
    // SAFETY: the asm template is a comment: it emits no instruction. It
    // names one general-purpose register as input and output, so the
    // register keeps `x`. `nomem`: no memory access. `nostack`: no stack
    // use. `preserves_flags`: flags untouched. `pure`: no side effects,
    // so it may be removed if `x` is unused (that is fine: an unused mask
    // cannot select anything). Audited in AUDIT.md for x86_64 and aarch64.
    #[allow(unsafe_code)]
    unsafe {
        core::arch::asm!("/* {0} */", inout(reg) x, options(pure, nomem, nostack, preserves_flags));
    }
    x
}

/// `bit` in {0, 1} (only bit 0 is used) to a mask: 0 or all-ones, made
/// opaque by [`value_barrier_u64`].
#[inline(always)]
#[must_use]
pub fn mask_from_bit(bit: u64) -> u64 {
    value_barrier_u64(0u64.wrapping_sub(bit & 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_and_masks() {
        for x in [0u64, 1, 2, u64::MAX, 0x8000_0000_0000_0000, 0x0123_4567_89ab_cdef] {
            assert_eq!(value_barrier_u64(x), x);
        }
        assert_eq!(mask_from_bit(0), 0);
        assert_eq!(mask_from_bit(1), u64::MAX);
        assert_eq!(mask_from_bit(2), 0);
        assert_eq!(mask_from_bit(3), u64::MAX);
    }
}
