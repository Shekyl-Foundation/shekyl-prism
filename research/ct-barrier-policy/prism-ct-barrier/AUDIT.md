# prism-ct-barrier: audit notes

Status: research candidate, not adopted. If Rick approves the policy
exception, this file is the template for the reviewed version.

## Scope of the exception

- One crate, `prism-ct-barrier`, with no dependencies.
- One `unsafe` block, in `value_barrier_u64`. The crate root has
  `#![deny(unsafe_code)]`; the block carries the only
  `#[allow(unsafe_code)]`. Every other shekyl-prism crate keeps
  `#![forbid(unsafe_code)]` and depends on this crate for masks.
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

1. `asm_scan2.py` on the release artifact (x86-64 and aarch64): no
   integer division, no new register-compared branches in the reduction.
2. `trace_check.py` under qemu-user: identical control-flow trace (guest
   PC sequence) across fixed, random and adversarial inputs, on both
   arches.
3. Valgrind memcheck taint (x86-64): no secret-dependent branch or
   address in the constant-time code; positive controls flagged.
4. dudect fixed-vs-random on native hardware (x86-64 here; aarch64 needs
   real hardware, not qemu).

## Review checklist for a change to this crate

- [ ] Still exactly one `unsafe` block; `#![deny(unsafe_code)]` intact.
- [ ] Template still emits no instruction on each supported arch
      (`objdump` of a test binary).
- [ ] No new public function; no dependency added.
- [ ] Options unchanged, or each change justified here.
- [ ] Checks 1–4 above re-run and recorded.
