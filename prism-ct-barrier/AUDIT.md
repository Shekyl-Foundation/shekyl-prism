# prism-ct-barrier: audit notes

Implementation of the G2 exception in SIGNER_SPEC §0. The measurements below describe the original isolated barrier experiment.

## Scope of the exception

- One crate, `prism-ct-barrier`, with no dependencies.
- One `unsafe` block, in `value_barrier_u64`. The crate root has
  `#![deny(unsafe_code)]`; the block carries the only
  `#[allow(unsafe_code)]`. The signer integer crate uses this mask. The verifier stays portable.
  The existing benchmark counter has a separate override.
- Two target architectures, `x86_64` and `aarch64`. Any other target is
  a `compile_error!`, so a new target needs a new audit, not a silent
  best-effort fallback (the failure mode of RUSTSEC-2026-0003 on ARM32).
- Public API: `value_barrier_u64(u64) -> u64` and
  `mask_from_bit(u64) -> u64` (0 or all-ones from bit 0). Nothing else.

## The block

```rust
core::arch::asm!("/* {0} */", inout(reg) x, options(pure, nomem, nostack, preserves_flags));
```

| property | why it holds |
|---|---|
| Emits no instruction | The template is an assembler comment (`/* */` is accepted by the GNU and LLVM assemblers for both x86-64 AT&T syntax and AArch64). Verified in the disassembly: the register simply carries the value. |
| Memory safety | `nomem`: no memory read or write. No pointers involved. |
| Stack | `nostack`: no push, no red-zone use. |
| Flags | `preserves_flags`: the template changes no flags (it is empty). |
| Register | `inout(reg)`: one general-purpose register in and out. The value is unchanged, so `value_barrier_u64` is the identity, which is the only functional property callers depend on. |
| `pure` | Allowed because the block has no side effects and its output depends only on its input. It lets LLVM delete an unused barrier or merge two identical ones, never look inside. |
| Opaqueness | LLVM treats inline asm as an opaque call: no value-range, known-bits or select-to-branch reasoning crosses it. This is the property `core::hint::black_box` only promises on a best-effort basis. |

Undefined behaviour: none possible from the template (it is empty); the
operand constraints are the standard ones.

## What the barrier does not guarantee

- It does not make the surrounding code constant time. The mask is opaque
  at the point of the barrier only. Later code can still branch on a
  value derived from the mask if it is written that way, or if the
  compiler re-derives a range from other information.
- It says nothing about instruction latency (e.g. `div`), memory access
  patterns, or microarchitectural effects (DIT/DOIT modes on aarch64 and
  Intel are not enabled here).
- It is not a guarantee across future compilers in the sense of a
  language-level contract; it relies on LLVM's long-standing treatment
  of inline asm as opaque, which the Rust reference documents ("the
  compiler cannot assume anything about the asm block beyond its
  declared operands and options").

## Required checks (every toolchain bump, every target)

1. Run `python3 scripts/ci/check_ct_shift.py --assembly shift.asm` on both
   supported Linux architectures. Inspect the release listing of the
   consuming functions, including the fixed 128-by-64 division.
2. The same command compares QEMU guest block sequences between entry and
   exit, including callees, across boundary cases. Its positive control
   must produce distinct traces.
3. On x86-64 the command also taints both arguments under Memcheck and
   requires a conditional-jump diagnostic on the positive control. It also
   taints the coordinates and norm throughout the P500 fixture reductions. Extend
   this coverage to the integrated signer before discharging G7.
4. dudect fixed-vs-random on native hardware (x86-64 here; aarch64 needs
   real hardware, not qemu).

## Review checklist for a change to this crate

- [ ] Still exactly one `unsafe` block; `#![deny(unsafe_code)]` intact.
- [ ] Template still emits no instruction on each supported arch
      (`objdump` of a test binary).
- [ ] No new public function; no dependency added.
- [ ] Options unchanged, or each change justified here.
- [ ] Checks 1–4 above re-run and recorded.
