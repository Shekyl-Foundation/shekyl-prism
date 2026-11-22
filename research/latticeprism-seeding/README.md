# research/latticeprism-seeding

**This is a research experiment for Shekyl PRISM, Rick Dawson's isogeny
signature scheme. Its code, data and conclusions, including negative
ones, are public by design.** It is not part of any crate, CI does not
build or test it, and it makes no constant-time or security claim.
Findings are in [REPORT_SEEDING.md](REPORT_SEEDING.md).

## Purpose

The 4-D experiment in [`../latticeprism-4d/`](../latticeprism-4d/) found
that H2 — a hash that *selects which* ideal of norm T to reduce — breaks
the 2-D-style target dependence, but a seed of H(pk) or H(pk ‖ T) is
fully recomputable by anyone who knows the public key. The question here
is which seed construction, if any, is **deterministic** and has a
**distribution indistinguishable from uniform**, without being publicly
precomputable.

"Deterministic" and "truly random" can only coexist as
pseudorandom-from-secret. This harness compares four ideas, plus the
failed public baseline, all feeding the same H2 selector.

## Constructions

| Id | Seed and challenge | Extra bytes in the signature |
|---|---|---|
| B0 | seed = H(pk ‖ q), q = FS(pk, msg). The 4-D baseline. | none |
| Hpk | seed = H(pk ‖ msg), q = FS(pk, msg). Public inputs only. | none |
| Hsk | seed = H(sk ‖ msg ‖ q), q = FS(pk, msg). EdDSA-style. | none |
| COM | r = H(sk ‖ msg), c = H(r) published, q = FS(pk, c, msg), seed = r. | 32-byte c |
| EPH | (esk, epk) = H(sk ‖ msg), epk published, seed = H(epk ‖ msg), q = FS(pk, epk, msg). | 32-byte epk |
| REV | Sample a unimodular U from H(sk ‖ msg) first, d = H(U) published, q = FS(pk, d, msg), then H2 with seed = H(sk ‖ msg ‖ q). | 32-byte d |

The 4-D H(pk)‖T failure is B0. Hpk is included to show what a public
fold does. REV's signing-ideal seed is the same formula as Hsk, but q is
bound to a commitment, as in COM. Report §4 says whether that is a real
difference or a relabelling.

## Running

Same Sage 10.x environment as `../cornacchia-harness/` and
`../latticeprism-4d/` (gmpy2, numpy, scipy, pandas, matplotlib).

```sh
# Full panels (P248/P324/P376/P500: 50 keys × 200 msgs). The runner
# skips any stream/prod/fixedq CSV that already exists.
micromamba run -n sage python run_seeding.py \
    --keys 50 --msgs 200 --keys-prod 50 --msgs-prod 200 \
    --fixed-q 24 --fixed-draws 150 --procs 7 --out results/raw

# P664 was run smaller (25 keys × 64 msgs, 16 × 40 fixed-q) so the
# leftover work after an interruption would finish in about 40 minutes.
micromamba run -n sage python analyze_seeding.py
```

## Sample sizes

- P248, P324, P376: 50 keys × 200 messages (stream and I_sec·I_q
  product), plus 24 fixed challenge degrees × 150 keys.
- P500: the same stream size; product 50 × 200; fixed-q 24 × 50.
- P664: 25 keys × 64 messages (stream and product); fixed-q 16 × 40.

## Reference papers

Same list as `../latticeprism-4d/README.md`: ePrint 2023/807 (the 2-D
control), TCHES 2025 constant-time dimension-4 LLL, ePrint 2025/2192
(constant-time quaternion arithmetic), and ePrint 2026/1169
(randomizable keys).
