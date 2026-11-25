# Constant-time value barrier vs `#![forbid(unsafe_code)]` (research)

Research only, on its own branch; nothing here changes the signer, the
signature format or any parameter set. PRISM is the scheme of ePrint
2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux,
Maino, Vercauteren, Wesolowski); shekyl-prism implements it.

## Question

The constant-time Z[i]-Lagrange prototype (`research/rust-lagrange-ct`)
needs an inline-asm value barrier (`unsafe { asm!(..) }`) in `src/ct.rs`:
its first barrier, `core::hint::black_box` on the choice bit, compiled
into a secret-dependent branch. Production crates are
`#![forbid(unsafe_code)]`. Is there a safe construction that holds up on
the pinned Rust 1.97.1, on x86-64 and aarch64, or should the project
approve one narrowly scoped, audited `unsafe` exception crate?

## Layout

| path | what |
|---|---|
| `harness/` | Copy of `research/rust-lagrange-ct` (same package and lib name, so symbol names match) with the value barrier made swappable: one `b-*` cargo feature per candidate, see `harness/Cargo.toml` and `harness/src/ct.rs`. Adds `rlc one` (one reduction, for trace comparison) and `RLC_FAST=1` (skip variable-time baselines, for qemu). |
| `prism-ct-barrier/` | The candidate exception crate: one `unsafe` asm block (x86_64, aarch64), `compile_error!` on any other arch. Audit notes in `prism-ct-barrier/AUDIT.md`. |
| `REPORT.md` | Results and the policy recommendation. |
| `results/` | Matrix outputs (asm scan, qemu trace hashes, memcheck, dudect). |

Vectors come from `research/rust-lagrange-ct/gen_vectors.py` and
`gen_adversarial.py` (deterministic) and are copied into
`harness/vectors/` (not committed).

Sets: P248, P324, P376, P500, P664 only.
