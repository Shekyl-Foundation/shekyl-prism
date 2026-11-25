# Constant-time value barrier vs `#![forbid(unsafe_code)]`: report

Date: 2026-11-25. Branch `experiment/ct-barrier-policy`, folder
`research/ct-barrier-policy/`. PRISM is the signature scheme of ePrint
2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux,
Maino, Vercauteren, Wesolowski); `shekyl-prism` implements it. Research
code: standalone crates outside the workspace and CI. Nothing here
changes the signer, the signature format, or any parameter set.

## Summary

- **Recommendation: approve one narrowly scoped, audited `unsafe`
  exception crate (`prism-ct-barrier`),** keeping
  `#![forbid(unsafe_code)]` on every other library crate (`prism-int`,
  quaternion/ideal arithmetic, etc.) and enforcing the boundary with
  the workspace's existing `scripts/ci/check_workspace_lints.py`
  mechanism (which already isolates `prism-bench`'s cycle-counter
  exception).
- **Why not "safe-only"?**
  1. `core::hint::select_unpredictable` (`selunpred`, Rust >= 1.88) fails
     catastrophically: 87 secret-dependent branch contexts in Valgrind
     memcheck, divergent control-flow traces on `x86-64` and `aarch64`,
     and `dudect max |t| = 30.00` at P248.
  2. `core::hint::black_box` on the choice bit (`weak`) fails in `1.97.1`
     just as it did in `1.85.1`: LLVM sees through `mask = 0u64.wrapping_sub(bb & 1)`
     and turns the barrel shifter (`ct::shr`) into conditional branches
     (20 memcheck branch contexts, divergent traces on `P248–P376`
     x86-64 and `P248–P500` aarch64, `dudect max |t| = 9.11` at P248).
  3. Safe `black_box` on the finished mask (`bbmask`) and
     `subtle` + `core_hint_black_box` (`subtle-bb`) survive the full
     `rust-lagrange-ct` reduction across our 4 build configs on `1.97.1`
     and `1.85.1`, **but** `bbmask` breaks in our tighter 7-target
     litmus kernels (`k_barrel` emits `b.eq` skipping vector stores on
     `aarch64` under `1.85.1`, and `k_cmov_u8` emits `beq` on `thumbv6m`),
     `bbuse` breaks in `k_barrel` on `x86-64`, `aarch64`, and `riscv`
     (plus slows P500 by ~23%: `11.08M` vs `9.00M` cycles), and
     `core::hint::black_box` is explicitly documented as best-effort
     only ("must not be relied upon to control critical program behavior").
  4. `prism-int` on `dev` (`26ce141`) **already** uses `unsafe` inside
     an external dependency: `subtle = "=2.6.1"` (`default-features = false`)
     implements its barrier as `#[inline(never)]` + `unsafe { core::ptr::read_volatile(&input) }`.
     That pays a ~7–12% cycle penalty at P248 (`2.10M` vs `1.88M`
     cycles for `crate`) from stack spill/reload per mask, and `subtle`
     is not audited by Shekyl.
- **Why `prism-ct-barrier` (`crate`)?** Zero instruction overhead
  (empty `/* {0} */` asm comment), zero external dependencies, matches
  the in-crate `asm` barrier identically (`P248` median `1.88M`, `P500`
  `9.00M` cycles, `4/4` clean on x86-64 and `4/4` clean on aarch64
  under `1.97.1`, `0` branch/address memcheck errors, `dudect max |t| =
  2.18 / 1.80`), emits **0** excess branches in all 7-target litmus
  kernels on supported targets, and hard-fails with `compile_error!` on
  any unaudited `target_arch` rather than silently falling back to a
  best-effort barrier.
- **Q5 context (`research/q5-width-proof`).** `prism-int::SecretInt` on
  `dev` currently supports only public-count shifts. Adding Q5's 64-bit
  quotient path requires secret `bit_length`, a secret-amount `shr`
  barrel shifter (the exact site where `weak` and litmus `bbmask`/`bbuse`
  broke), and sticky overflow marking if a quotient clamps (since Q5
  found clamping can stall reduction on crafted ideals). Placing
  `prism-ct-barrier` beneath `prism-int` closes this hazard before stage
  2c lands.

## 1. Candidates tested

Exact toolchain pin on `dev`: `rust-toolchain.toml` specifies
`channel = "1.97.1"`. Both `1.97.1` and `1.85.1` (the `research/rust-lagrange-ct`
box compiler) were tested.

| Candidate | Mechanism | Where `unsafe` lives | Notes |
|---|---|---|---|
| `asm` | `asm!("/* {0} */", inout(reg) x)` in `ct::mask` | Signer crate | Baseline from `research/rust-lagrange-ct` |
| `crate` | `prism_ct_barrier::mask_from_bit` (`#[allow(unsafe_code)]` on 1 fn) | Isolated internal crate (`prism-ct-barrier`) | `compile_error!` outside `x86_64` / `aarch64`; see `prism-ct-barrier/AUDIT.md` |
| `none` | Plain `0u64.wrapping_sub(bit & 1)` | None (safe) | Negative control |
| `weak` | `0u64.wrapping_sub(black_box(bit) & 1)` | None (safe) | Negative control (first barrier that broke in `rust-lagrange-ct`) |
| `bbmask` | `black_box(0u64.wrapping_sub(bit & 1))` | None (safe) | Safe `black_box` on finished mask |
| `bbuse` | Plain mask; `black_box(m)` at every `select`/`cswap`/div use | None (safe) | Safe `black_box` at consumers |
| `noinline` | `#[inline(never)] fn mask(bit)` | None (safe) | Relies on non-inlining / no IPA specialization |
| `selunpred` | `core::hint::select_unpredictable(bit & 1 == 1, u64::MAX, 0)` | None (safe, Rust >= 1.88) | Perf hint; docs disclaim CT |
| `subtle-bb` | `subtle 2.6.1` with `core_hint_black_box` feature | None (safe) | Matches `subtle` without `unsafe` |
| `subtle-vol` | `subtle 2.6.1` default (`#[inline(never)] read_volatile`) | Inside `subtle` | What `prism-int` on `dev` currently pulls in |
| `volatile` | `core::ptr::read_volatile(&m)` inline in `ct::mask` | Signer crate | Inline volatile without `#[inline(never)]` |
| `cmov` | `cmov 0.5.4` (`Cmov::cmovnz`) | Inside `cmov` (`asm!` cmov/csel) | External RustCrypto crate; never audited (per its README) |
| `ctutils` | `ctutils 0.4.8` (`Choice::from_u64_lsb` + `CtSelect`, Rust >= 1.87) | Inside `cmov` | Built on `cmov`; Rust 2024 / MSRV 1.87 |

## 2. Recommendation table (candidate × toolchain × arch × asm-clean × taint × dudect)

- **asm-clean**: fraction of checked build configs (`c1`–`c4` on `1.97.1`;
  `c1, c3` on `1.85.1`) whose `qemu-user` control-flow trace inside
  `zi::reduce` and `ct::exact_round` is identical across fixed, random,
  and adversarial inputs (`c1` tests all 5 sets, `p664` with 3 inputs,
  others with 6; `c2/c3/c4` test `p248` and `p500` with 2–3 inputs), with
  all outputs correct and no secret-data division. (`ok*` on `aarch64 c4`
  `neoverse-v1` marks public SVE vector-length `udiv` `N / rdvl`, see §3.)
- **taint**: Valgrind memcheck (`ctgrind` method, `x86-64`, `c1`,
  statically linked, 5 sets on `1.97.1`, 3 sets on `1.85.1`).
  `0 (clean; N shift)` means 0 branch/address taint errors and `N`
  size-1 count warnings on fixed-latency SIMD shifts (`psrlq`/`psllq` in
  `ct::shr`), verified by disassembly (§5).
- **dudect**: Welch `|t|` max over raw, 100 percentile crops, and 2nd-order
  centred squares on `x86-64` (`1.97.1`, `c1`, core-pinned), `40,000`
  traces at P248 and `20,000` at P500. Threshold `|t| = 4.5`.

| Candidate | `unsafe` where | Toolchain | x86-64 asm-clean | aarch64 asm-clean | x86-64 taint (memcheck) | x86-64 dudect max\|t\| (P248 / P500) |
|---|---|---|---|---|---|---|
| `crate` | isolated in `prism-ct-barrier` | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 25 shift) | 2.18 / 1.80 |
| `crate` | isolated in `prism-ct-barrier` | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `asm` | in signer crate | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 25 shift) | 1.53 / 2.07 |
| `asm` | in signer crate | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `cmov` | inside `cmov` dep | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 21 shift) | 1.80 / 2.03 |
| `cmov` | inside `cmov` dep | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `ctutils` | inside `cmov` dep | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 23 shift) | 2.54 / 1.57 |
| `ctutils` | inside `cmov` dep | `1.85.1` | n/a (MSRV 1.87) | n/a (MSRV 1.87) | build failed | not run |
| `subtle-vol` | inside `subtle` dep | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 18 shift) | 2.08 / 1.70 |
| `subtle-vol` | inside `subtle` dep | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `volatile` | in signer crate | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 28 shift) | 2.44 / 1.72 |
| `volatile` | in signer crate | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `bbmask` | safe | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 24 shift) | 1.59 / 1.84 |
| `bbmask` | safe | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean; 6 shift) | not run |
| `bbuse` | safe | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 22 shift) | 1.62 / 2.05 |
| `bbuse` | safe | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `noinline` | safe | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 22 shift) | 2.31 / 2.12 |
| `noinline` | safe | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `subtle-bb` | safe | `1.97.1` | 4/4 clean | 4/4 clean | 0 (clean; 22 shift) | 1.85 / 2.94 |
| `subtle-bb` | safe | `1.85.1` | 2/2 clean | 2/2 clean | 0 (clean; 6 shift) | not run |
| `weak` | safe (neg. ctrl) | `1.97.1` | 0/4 clean (c1,c2,c3,c4) | 1/4 clean (c1,c3,c4) | **20 branch errors** | **9.11** / 1.83 |
| `weak` | safe (neg. ctrl) | `1.85.1` | 0/2 clean (c1,c3) | 0/2 clean (c1,c3) | **14 branch errors** | not run |
| `selunpred` | safe (Rust >= 1.88) | `1.97.1` | 0/4 clean (c1,c2,c3,c4) | 1/4 clean (c1,c3,c4) | **87 branch errors** | **30.00** / 2.79 |
| `selunpred` | safe (Rust >= 1.88) | `1.85.1` | n/a (Rust 1.88) | n/a (Rust 1.88) | build failed | not run |
| `none` | safe (neg. ctrl) | `1.97.1` | 0/4 clean (c1,c2,c3,c4) | 1/4 clean (c1,c3,c4) | **81 branch errors** | **32.36** / 2.10 |
| `none` | safe (neg. ctrl) | `1.85.1` | 0/2 clean (c1,c3) | 0/2 clean (c1,c3) | **55 branch errors** | not run |

## 3. Build-config sweep and SVE `udiv` observation

Configurations:
- `c1`: `opt-level=3, lto=fat, codegen-units=1`
- `c2`: `opt-level=2, lto=fat, codegen-units=1`
- `c3`: `opt-level=3, lto=off, codegen-units=16`
- `c4`: `opt-level=3, lto=fat, codegen-units=1`, plus `-C target-cpu=x86-64-v3`
  (x86-64: AVX2/BMI2/ADX) or `-C target-cpu=neoverse-v1` (aarch64).

Full per-config verdict matrix is in `results/tables.md`. Key points:
- On `aarch64 c2` (`opt-level=2`), even `none`, `weak`, and `selunpred`
  kept `csel` and showed identical traces — meaning `opt-level=2` masks
  barrier weakness that reappears at `opt-level=3` (`c1`, `c3`, `c4`).
- On `x86-64`, `none`, `weak`, and `selunpred` diverged at `P248–P376`
  (`N = 22..32` limbs) across all four configs, while `P500` and `P664`
  (`N = 42, 54`) kept a single trace (matching dudect: `P248` leaked at
  `max |t| = 9.11..32.36`, `P500` did not). At `P248`, `addr2line` on the
  divergent PC in `weak` locates the exact branch at `ct::select_arr::<22>`
  inside `ct::shr::<22>` (`ct.rs:326` via `zi::quotient`, `zi.rs:67–68`).
- On `aarch64 c4` (`neoverse-v1`), `asm_scan2.py` flagged `udiv` in
  `p500:reduce` (1), `p664:exact_round` (2), and `p664:reduce` (49).
  Disassembly (`aarch64-linux-gnu-objdump`) confirmed these are **ARM SVE**
  auto-vectorization trip counts (`mov w8, #0x36; rdvl x14, #2; udiv x8, x8, x24`),
  dividing the public limb count `N` by the hardware SVE vector length,
  not secret data. Every non-failing barrier had `cf_equal = true` on
  `neoverse-v1`.
- `-C target-cpu=native` on this Xeon host emits AVX-512 instructions
  that neither `qemu-x86_64` nor Valgrind on this box decodes, so `c4`
  used `x86-64-v3` for traced execution. Static `asm_scan2.py` on the
  saved AVX-512 `native` binaries (`1.97.1`) showed `asm` and `crate`
  are identical (`br 1002 reg 73 sel 9574 div 0`), while `weak` adds +10
  branches (`1012`) and `none` alters control flow (`br 889 reg 83`).

## 4. 7-target litmus hazard suite (`litmus/`)

To test whether barriers hold under tighter unrolled patterns (where
LLVM more aggressively proves masks are `{0, -1}`), six small `#[no_mangle]`
kernels (`k_dalek_sub` [RUSTSEC-2024-0344 shape], `k_barrel`, `k_cswap`,
`k_cmov_u8` [RUSTSEC-2026-0003 shape], `k_table`, `k_cond_sub`) were
compiled with `opt-level=3, codegen-units=1` across 7 targets (`x86_64`,
`aarch64`, `armv7`, `thumbv7em`, `thumbv6m`, `riscv64`, `riscv32`) on
both `1.97.1` and `1.85.1`. Table reports excess conditional branches
above the reference (`crate` on x86-64/aarch64; `cmov` elsewhere); full
matrix in `results/litmus.md`.

Spot-checking the excess branches in assembly distinguishes two causes:
1. **Genuine secret-dependent branches:**
   - `none`, `weak`, `selunpred`: branches across 4–6 kernels on all
     targets.
   - `bbmask` (safe): **+3** in `k_barrel`/`k_table` on `1.85.1 aarch64`
     (e.g. `b.eq .LBB1_4` skipping vector stores in `k_barrel`) and
     **+1** in `k_cmov_u8` on `thumbv6m` (`cmp r2, #0; beq .LBB1_2` on
     both `1.97.1` and `1.85.1`).
   - `bbuse` (safe): **+3** in `k_barrel` on `x86_64` (`cmpq $1, %rcx; ja`),
     **+3/+6** on `aarch64`, **+1** on `thumbv6m`, **+1..+3** on `riscv64`/`riscv32`.
   - `volatile` (inline `read_volatile`): **+1** in `k_cmov_u8` on `thumbv6m`.
2. **Public loop-counter non-unrolling (not a secret branch):**
   - `subtle-bb` (`+1` on `x86_64`, `1.85.1 aarch64`), `ctutils` (`+1` on
     `x86_64`, `aarch64`), and `cmov` (`+1` on `1.85.1 aarch64`) had `+1` in
     `k_table` solely because LLVM kept the public `for i in 0..16` loop
     (`cmp x13, #16; b.ne`) rather than unrolling it as it did with `crate`.
3. **Zero excess branches everywhere built:** `crate` (`x86_64`,
   `aarch64`; `compile_error!` elsewhere), `asm` (`x86_64`, `aarch64`,
   `riscv64`), `noinline`, `subtle-vol`.

## 5. Performance and memcheck details

- **Cycle cost (`dudect` medians, `1.97.1 c1`, x86-64, P248 / P500):**
  - `crate`: **1.88M / 9.00M** cycles (matches `asm`: 1.88M / 9.19M —
    zero barrier instruction overhead).
  - `bbmask`: 1.82M / 9.53M.
  - `subtle-bb`: 1.92M / 10.08M (+12% at P500).
  - `cmov`: 2.07M / 9.65M; `ctutils`: 1.93M / 10.13M.
  - `volatile`: 2.02M / 9.42M; `subtle-vol`: 2.10M / 9.98M (+7–12% at
    P248 from stack store/load per mask).
  - `noinline`: 2.17M / 10.12M (+12–15% call overhead).
  - `bbuse`: 2.00M / **11.08M** (+23% at P500, blocks CSE/regalloc).
- **Memcheck (`ctgrind`):** In `1.97.1`, LLVM vectorized the remainder
  of `ct::shr` using `psrlq %xmm6, %xmm2` and `psllq %xmm0, %xmm4`,
  where the shift count is the secret sub-word shift `bs = k & 63`.
  Valgrind memcheck flags `Use of uninitialised value of size 1` on any
  SIMD shift with an undefined shift-count register (18–28 contexts
  across all clean candidates on `1.97.1`). Disassembly verified those
  sites are all `psrlq`/`psllq` with `%xmm` count (data-independent
  latency on x86-64) and `0` branches (`branch=0, other=0`), while
  positive controls `none` (81), `weak` (20), `selunpred` (87), and
  `asm_control` (1 in `rlc`) were all flagged as `Conditional jump or move`.

## 6. Policy recommendation for `shekyl-prism`

Adopt **`prism-ct-barrier`** (the isolated exception crate, see
`prism-ct-barrier/AUDIT.md`) rather than relying on safe-only barriers:

1. **No honest "safe-only" alternative exists on `dev` anyway.** `prism-int`
   already depends on `subtle = "=2.6.1"` (`default-features = false`),
   which uses `unsafe { core::ptr::read_volatile }` inside an external
   un-audited crate. Enabling `subtle/core_hint_black_box` swaps that
   for a `core::hint::black_box` hint that Rust explicitly warns must
   not be relied upon for critical behavior, and `bbmask`/`bbuse`
   already broke in our litmus suite under LLVM optimizations.
2. **Governance already exists.** `scripts/ci/check_workspace_lints.py`
   already enforces `unsafe_code = "forbid"` on all crates except a
   single audited exception (`prism-bench::counter` with `unsafe_code = "deny"`
   and one `#[allow(unsafe_code)]`). Adding `prism-ct-barrier` to that
   allowlist with the exact same one-block restriction keeps every
   cryptographic crate (`prism-int`, stage 2b–5 crates) at
   `#![forbid(unsafe_code)]`.
3. **Why a barrier alone is not a proof.** Even with `prism-ct-barrier`,
   an optimizer can only be stopped at the barrier site; downstream code
   or future LLVM versions can still introduce branches or `udiv`. The
   constant-time feasibility gate in `SIGNER_SPEC.md` should require
   running `asm_scan2.py`, `qemu-user` trace equivalence (`x86-64` and
   `aarch64`), and Valgrind `ctgrind` (`x86-64`) in CI/nightly on every
   toolchain bump, plus `dudect` on native x86-64 and ARM64 hardware.

## 7. Caveats

- **Emulated ARM64 (`qemu-aarch64`).** All `aarch64` runtime checks ran
  under `qemu-aarch64` (`10.0.13`) on an x86-64 host. `trace_check.py`
  verifies control-flow PC sequence equivalence and arithmetic
  correctness, **not** ARM64 microarchitectural timing or memory
  addresses; timing under QEMU is meaningless. Native ARM64 `dudect`
  still needs real ARM64 hardware.
- **VM timing (`x86-64`).** `dudect` ran pinned to one core using
  `lfence; rdtsc; lfence` on a shared hypervisor VM (`2.4 GHz` invariant
  TSC) with no PMU instruction-retired counter.
- **Secondary config scope.** Config `c1` (`1.97.1`) traced all 5
  parameter sets (`p664` on 3 inputs, others on 6); `1.85.1 c1` traced
  `p248, p376, p500` (6 inputs); secondary configs `c2, c3, c4` traced
  `p248, p500` on 2–3 inputs (`fixed`, `a0` [, `p1`]), and `1.85.1`
  configs `c2, c4` were built without running `qemu` traces.
- **Simulated inputs.** The harness runs `rust-lagrange-ct`'s index-4-order
  inputs, not full native PRISM keygen/signing yet.
