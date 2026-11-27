# A fixed-count divstep (safegcd) modular inverse for `prism-int` widths

Rick's t56u, following the literature sweep (`../literature-sweep-2026-11/`, item on the
EUROCRYPT 2026 safegcd bound). Research only: no format changes, nothing in the
workspace is modified. The crate here is standalone (`[workspace]` in its own
`Cargo.toml`) and builds against dev's `prism-int` at `8232184`.

**No constant-time claim.** The prototype is written branch-free, with fixed counts and
the signer's audited value barrier, and the compiled loop was inspected on x86-64 and
AArch64 (§5). That is an engineering check, not a constant-time result: no dudect,
Valgrind taint or trace-equivalence run was done.

## 1. Where the code inverts today (dev `8232184`; `research` and `feat/stage2b-2d-arithmetic` are the same at these sites)

| site | modulus | secret? | width b (P248 / P324 / P376 / P500 / P664) | method today |
|---|---|---|---|---|
| `prism-int/src/number.rs:148` `Int::inv_mod` | any | n/a | n/a | Euclid, variable time. **No library caller** on dev, research or feat: only `prism-int/tests/oracle_number.rs` and `tests/vectors.rs` call it. |
| `shekyl-prism/src/ideal/mod.rs:357–365` (`intersection`) | q, the challenge degree, prime, public | the operand `norm(ρ(v))` is secret | 491 / 643 / 747 / 995 / 1323 (`2e − 5`) | Fermat, `norm^(q−2)` in Montgomery form, ~b squarings + ~b/2 multiplications, then the `inv·norm = 1` check (line 365). Already branch-free on the secret. |
| `shekyl-prism/src/ideal/mod.rs:197, 267` `inverse_odd` | 2^(64W) | public modulus | 704 … 1728 | 2-adic Newton for the Montgomery constant. Power-of-two modulus: safegcd does not apply and is not needed. |
| C reference `qlapoti.c:635` `ibz_invmod(a_alpha, n)` (the CT redesign's `λ⁻¹ mod n` per slot) | n, the qlapoti target norm | **secret** | bound 128 / 165 / 194 / 255 / 337 (`(L+1)/2 + 2`) | GMP, variable time. Not yet ported. **About half of the n are even**, which safegcd (odd modulus) cannot take directly; see §6. |
| Fp / Fp² inversion | p | — | 251 … 669 | the `fp2` crate's own binary gcd; out of scope. |

No inverse is taken modulo the secret key norm N, Nq or 2N anywhere; their widths are
in `bounds.txt` for completeness (N: 158 / 195 / 224 / 285 / 367 bits; Nq: 649 / 838 /
971 / 1280 / 1690; 2N: 159 / 196 / 225 / 286 / 368). Powers of two (2^(64W), 2^f, 2^e)
are Newton's job.

So the inverse that matters is **q, in the signer's intersection**, against Fermat,
and **n, in a future qlapoti port**, where the alternative is a variable-time gcd.

## 2. Bounds (recomputed: `bounds.py` → `bounds.txt`)

- BY2019 (Bernstein–Yang, δ₀ = 1, |f|, |g| ≤ 2^b, b ≥ 46): ⌊(49b + 80)/17⌋.
- EUROCRYPT 2026 Theorem 1 (machine-checked in HOL Light; δ₀ = 1/2, f odd, 0 ≤ g ≤ f ≤ 2^b): ⌈(9437b + 1)/4096⌉. Check: 590 at b = 256, as the paper states.

| set | class | b | 2019 divsteps | 2026 divsteps | ratio | 62-step jumps 2019 / 2026 |
|---|---|---|---|---|---|---|
| P248 | q | 491 | 1419 | 1132 | 0.798 | 23 / 19 |
| P248 | n | 128 | 373 | 295 | 0.791 | 7 / 5 |
| P248 | N | 158 | 460 | 365 | 0.794 | 8 / 6 |
| P324 | q | 643 | 1858 | 1482 | 0.798 | 30 / 24 |
| P324 | n | 165 | 480 | 381 | 0.794 | 8 / 7 |
| P324 | N | 195 | 566 | 450 | 0.795 | 10 / 8 |
| P376 | q | 747 | 2157 | 1722 | 0.798 | 35 / 28 |
| P376 | n | 194 | 563 | 447 | 0.794 | 10 / 8 |
| P376 | N | 224 | 650 | 517 | 0.795 | 11 / 9 |
| **P500** | **q** | **995** | **2872** | **2293** | 0.798 | **47 / 37** |
| **P500** | **n** | **255** | **739** | **588** | 0.796 | **12 / 10** |
| P500 | N | 285 | 826 | 657 | 0.795 | 14 / 11 |
| P664 | q | 1323 | 3818 | 3049 | 0.799 | 62 / 50 |
| P664 | n | 337 | 976 | 777 | 0.796 | 16 / 13 |
| P664 | N | 367 | 1062 | 846 | 0.797 | 18 / 14 |

Asymptotic ratio 0.7993. The 2026 hypothesis `0 ≤ g ≤ f` holds at both real sites after
reducing the operand mod the modulus (the q site already holds a residue in `[0, q)`
once out of Montgomery form). The n site has `g < f` only on the odd-n path; the
even-n reformulation in §6 inverts n modulo the operand, where **only the 2019 bound
applies** unless the operand is the larger.

## 3. Prototype (`src/lib.rs`)

libsecp256k1 `modinv64` structure at any width: signed 62-bit limbs `S62<K>` with
K = ⌈(b + 3)/62⌉ (q: 8 / 11 / 13 / 17 / 22; n: 3 / 3 / 4 / 5 / 6), a 62-divstep jump on
the low 64 bits producing a matrix with |u| + |v| ≤ 2^62, `update_fg`, and `update_de`
keeping d, e in (−2m, m) via m⁻¹ mod 2^62 (5-step Newton). δ is carried doubled (2δ
odd, starting at 1 for δ₀ = 1/2). The jump count is a caller-supplied public number
(`jumps(divsteps_2026(b))`). Final normalization: add m if negative, negate by the sign
of f, add m if negative. `ok` is all ones iff g = 0 and |f| = 1; for a non-unit the
value is zeroed. Every select is a mask; every mask passes `prism-ct-barrier`'s
`value_barrier_u64` (§5).

## 4. Correctness (`tests/oracle.rs`; `cargo test --release` and debug with overflow checks: 18 / 18 pass)

- **105,445 class cases** (44,878 invertible, 60,567 non-invertible; per-class tallies in `test-debug.txt`), every one checked against a BigInt `extended_gcd` oracle **and** `prism-int` `Int::<W>::inv_mod` at the set's W (all 105,445 compared). Per set and class (q, n, N): 4,000 random (2,000 for N); structured moduli 2^b − 1, 2^(b−1) + 1, 3, 5, 7, 0xfffffff1, random b- and b/2-bit, with operands 0, 1, 2, m − 1, m − 2, (m − 1)/2, (m − 1)/2 + 1 and 3…39; composite non-units; small-factor moduli.
- **Paper vector** (the paper's f, g needing exactly 590 divsteps at b = 256): the variable-time counter gives 590; the CT routine is correct with 10 jumps (620) and returns `ok = 0` with 9 jumps (558). The count is load-bearing.
- **Exhaustive** b = 1 … 11: all 1,400,148 pairs with odd f ≤ 2^b, 0 ≤ g ≤ f. Worst count ≤ bound everywhere, and equal to it at b = 5, 6, 8, 9.
- **Adversarial, forward** (`src/bin/adversarial.rs`, `adversarial.txt`): random sampling plus hill-climbing. Hill-climb max against the bound: P500 q 2037 / 2293, n 529 / 588, N 591 / 657; P664 q 2711 / 3049. Barely better than random (~0.89 of bound).
- **Adversarial, backward** (`backward.py 3100 100`, `backward.txt`, after the paper's §4 idea, with a crude max-norm and one beam per δ value): reaches **0.953–0.959 of the 2026 bound at every target width** (b = 256: 566 of 590; P500 q: 2193 of 2293; P664 q: 2917 of 3049). Test `backward_search_vectors` checks all 16 vectors: the var-time count matches, the CT routine inverts them at the bound's jump count, and with ⌊n/62⌋ jumps it does not finish. (A first version kept only the globally smallest states, drifted to δ values that can never be a start and found nothing; bucketing by δ fixed it. The paper's stable norm reaches 590 exactly; ours does not, so these are hard inputs, not worst cases.)

No input anywhere needed more divsteps than the 2026 bound.

## 5. Branch and stack audit (`src/bin/audit.rs`, objdump of `inv_mod_ct::<17>` and `::<5>`)

**Surprise: the first release build branched on `ok`.** LLVM saw that `ok` is 0 or −1 and
compiled `value &= ok` into `je` around the zeroing stores (x86-64 `f678: je`, `100d2: je`
at K = 5). That leaks invertibility only, but it is exactly the failure
`prism-ct-barrier` exists for. After routing every mask (`neg_mask`, the divstep `c2`,
and `ok`) through `value_barrier_u64`:

| target | K | instructions | conditional branches | all on |
|---|---|---|---|---|
| x86-64 | 17 | 998 | 4 | 62-step counter (`dec; jne`), two limb loops (`cmp $0x11`), the jump count |
| x86-64 | 5 | 791 | 2 | 62-step counter, jump count (limb loops unrolled) |
| aarch64 | 17 | 775 | 4 | 62-step counter (`subs w20,#1`), limb loops (`cmp x21,#0x80`, `cmp x19,#0x88`), jump count |
| aarch64 | 5 | 622 | 2 | 62-step counter, jump count |

No `cmov`/`csel` on data other than one `sete`/`csetm` producing `ok` (flag to register,
no branch); no division; calls only to `memcpy` of fixed size. Multiplies are `mul`/
`imul` (x86) and `mul`/`umulh`/`smulh`/`madd` (aarch64), fixed-latency on the targeted
cores. Frame: 0x298 (664 B) + 6 pushes at K = 17 on x86-64, 0x2d0 (720 B) on aarch64;
0xb0 (176 B) at K = 5. Far inside the signer's 32 KiB stack budget.

## 6. Cost (`src/bin/bench.rs`; ns per inverse, median of rounds over 64 inputs; includes u64 ↔ S62 conversion)

The Fermat column is dev's `Modulus::mul` and the intersection's `q − 2` ladder copied
verbatim on a random odd q of exactly 2e − 5 bits (cost depends on length and popcount,
not primality). `Int::inv_mod` runs at the signer width W.

**x86-64** (box, Xeon VM, shared; `bench-x86.txt`)

| set | class | jumps | divstep | 2019-count divstep | `Int::inv_mod` | Fermat | Fermat / divstep |
|---|---|---|---|---|---|---|---|
| P248 | q | 19 | 4.3 µs | 5.4 µs | 65 µs | 732 µs | 168× |
| P248 | n | 5 | 1.0 µs | 1.5 µs | 17 µs | | |
| P324 | q | 24 | 5.9 µs | 7.4 µs | 162 µs | 1.41 ms | 239× |
| P376 | q | 28 | 7.4 µs | 9.2 µs | 254 µs | 2.03 ms | 275× |
| **P500** | **q** | 37 | **9.9 µs** | 12.6 µs | 503 µs | **4.54 ms** | **456×** |
| **P500** | **n** | 10 | **2.1 µs** | 2.5 µs | 122 µs | | |
| P664 | q | 50 | 14.8 µs | 18.3 µs | 969 µs | 9.93 ms | 672× |
| P664 | n | 13 | 2.7 µs | 3.4 µs | 249 µs | | |

(P324 and P376 n rows are in the file: 1.5 µs and 1.6 µs.)

**Raspberry Pi 4, skl-pi4-1** (Cortex-A72, `shekyld` left running, load 2.0; binary in
`~/shekyl-timing/safegcd-bench`; `bench-pi4.txt`)

| set | class | divstep | 2019-count | `Int::inv_mod` | Fermat | Fermat / divstep |
|---|---|---|---|---|---|---|
| P248 | q | 20.7 µs | 24.9 µs | 420 µs | 4.10 ms | 198× |
| **P500** | **q** | **52.3 µs** | 66.2 µs | 2.98 ms | **27.0 ms** | **516×** |
| P500 | n | 9.4 µs | 11.3 µs | 743 µs | | |
| P664 | q | 80.9 µs | 101 µs | 6.06 ms | 57.9 ms | 715× |

The 2026 bound saves 20–27 % over the 2019 count, as the divstep ratio predicts.
The **big number is Fermat**: the intersection's inverse costs about 4.5 ms on x86 and
27 ms on the Pi at P500, against 10 µs and 52 µs. Part of that is the generic
`SecretInt` Montgomery multiply (≈ 3 µs per multiply at W = 21 on x86, ~1,500 multiplies) rather than Fermat as
such; but even a much faster multiply leaves ~1,500 of them against 37 jumps.

## 7. Residual risks

- Not a constant-time result (see top). Needs the same artifact checks G7 gets (dudect, Valgrind taint, trace equivalence), on both targets.
- Barrier dependence: without `value_barrier_u64` LLVM produced a data branch. Any port must keep the barriers and re-run the asm scan per toolchain bump.
- Preconditions not enforced in constant time: m odd and > 1, 0 ≤ x < m, m < 2^b for the b whose jump count is passed. Passing too few jumps yields `ok = 0`, not a wrong value, on every test, but that is not a proof.
- Montgomery-form inputs: at the intersection the operand is `norm·R mod q`; the result must be multiplied by R³ (one Montgomery multiply by a precomputed `R³ mod q`, or two by R²) to return to Montgomery form. Not prototyped.
- Even n (qlapoti): safegcd needs an odd modulus. Options: (a) write n = 2^k·n′, invert mod n′ by divsteps and mod 2^k by Newton, CRT; k is secret, so k must be handled as a mask over all positions, or (b) invert n modulo the (odd) operand a and set x = (1 − n·y)/a by 2-adic exact division, where g > f and only the 2019 bound holds. Neither is prototyped.
- The 2026 bound's hypothesis is 0 ≤ g ≤ f ≤ 2^b with f odd; the q site satisfies it; (b) above does not.
- The backward search uses a crude norm and reaches ~0.956 of the bound; the paper's exact worst case at 256 bits is the only bound-tight vector tested beyond b = 11.

## 8. Recommendation

1. **Intersection (q, public prime): replace the Fermat ladder with the divstep inverse**, keeping the existing `inv·norm = 1` check as the `ok` source (or AND both). This is the only site with a measured payoff: ~450× at P500 on x86, ~500× on the Pi, i.e. ~4.5 ms and ~27 ms saved per call. Use the 2026 count (37 jumps at P500) and the K = 17 limb layout; add the R³ conversion. This is an internal arithmetic change with no format effect, but it touches the signer's audited path, so it goes through Rick's design round and the G7 artifact checks before it lands.
2. **qlapoti port (n, secret, sometimes even):** use divsteps for the odd part when the port happens; decide (a) vs (b) for even n in that design. Budget 10 jumps (2.1 µs x86) at P500.
3. **`Int::inv_mod`:** leave it as is (no callers). If it ever gets a secret caller, route it to this routine.
4. Nothing to do for N, Nq, 2N, or the power-of-two moduli.

## Files

`bounds.py`/`bounds.txt` (bounds and widths), `src/lib.rs` (prototype), `src/rng.rs`,
`tests/oracle.rs` (18 tests), `test-debug.txt` (tallies), `src/bin/adversarial.rs` +
`adversarial.txt`, `backward.py` + `backward.txt`/`backward.log`, `src/bin/bench.rs` +
`bench-x86.txt`/`bench-pi4.txt`, `src/bin/audit.rs` (asm audit instances).
Reproduce: `cargo +1.97.1 test --release`, `cargo +1.97.1 run --release --bin bench 41`,
`python3 backward.py 3100 100`; aarch64 with
`CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc cargo +1.97.1 build --release --target aarch64-unknown-linux-gnu`.
