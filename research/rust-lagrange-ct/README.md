# Rust port of the fixed-schedule Z[i]-Lagrange reduction (research)

Research only. PRISM is the scheme of ePrint 2026/443 (Basso, Borin,
Castryck, Corte-Real Santos, Invernizzi, Leroux, Maino, Vercauteren,
Wesolowski); shekyl-prism implements it. This directory does not change
the signer, the signature format, or any parameter set. The crate here is
standalone: an empty `[workspace]` table keeps it out of the repository
workspace, so it is not built, linted or tested by CI. It uses `unsafe`
(rdtsc, CPU pinning, an inline-asm value barrier, Valgrind client
requests), which the workspace forbids; that is another reason it stays
out.

Results are in [REPORT.md](REPORT.md).

## Question

`research/fixed-schedule-reduction` (Python model) found that a
Z[i]-Lagrange reduction with a fixed step count T, sized from the public
bound B = ⌈L/2⌉+32 on the secret norm (T = 189/239/283/359/465 for
P248/P324/P376/P500/P664), finds λ₁ on every sample at about 0.28× exact
LLL's multiply-and-divide count. Step counts are not cycles. Does that
hold up as real constant-time code?

## Files

| file | what |
|---|---|
| `src/ct.rs` | Fixed-limb two's-complement integers and constant-time primitives: mask selects behind an inline-asm value barrier, borrow-chain comparisons, branch-free bit length, barrel shifter, fixed-iteration restoring division (no `div`). |
| `src/zi.rs` | The fixed-schedule Z[i]-Lagrange reduction (`reduce::<C, N, PL>`), with fsr.py's 64-bit bounded-precision quotient. |
| `src/sets.rs` | Per-set widths from public data: C = ⌈(bits(q)+B+3)/64⌉ coordinate limbs, N = 2C norm limbs. |
| `src/exact.rs`, `src/exact_gmp.rs` | Variable-time baselines: Cohen 2.6.7 exact LLL (δ = 99/100) and exact early-exit Z[i]-Lagrange, on num-bigint and on GMP (rug). |
| `src/bin/rlc.rs` | Driver: `check`, `bench`, `dudect`, `ctgrind`. |
| `gen_vectors.py` | Test vectors from the Python model (fsr.py/hdl.py unchanged): λ₁, the model's exact and 64-bit-quotient outputs. |
| `asm_scan.py` | Static scan of the compiled reduction. |
| `ctgrind/` | Valgrind memcheck as a constant-time checker (secrets marked undefined), with positive controls. |
| `analyze.py` | Tables and `results/summary.json`. |
| `results/` | `check.txt`, `bench.csv`, `tables.md`, `asm_scan.txt`, `ctgrind.txt`, `summary.json`, `dudect/*.csv.gz`. |

## Running

```sh
PY=/workspace/mamba/envs/sage/bin/python
FSR_DIR=../fixed-schedule-reduction JOBS=2 $PY gen_vectors.py 500 2000   # ~5 min, deterministic
cargo build --release -j2                                  # check/bench/dudect without GMP
./build_gmp.sh                                             # with the GMP baselines
./target/gmp/release/rlc check                             # cross-check, all sets
RLC_CPU=6 ./run_measure.sh                                 # bench + dudect, pinned
./ctgrind/run.sh                                           # memcheck constant-time check
$PY analyze.py > results/tables.md
```

Sets: P248, P324, P376, P500, P664 only.
