# research/latticeprism-seeding

**This is a research experiment for Shekyl PRISM. PRISM is the signature
scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin, Wouter Castryck,
Maria Corte-Real Santos, Riccardo Invernizzi, Antonin Leroux, Luciano
Maino, Frederik Vercauteren, and Benjamin Wesolowski; the paper is at
`docs/papers/2026-443_PRISM_with _salt.pdf`). Shekyl's `shekyl-prism` is
an implementation of it. The experiment's code, data and conclusions,
including negative ones, are public by design.** It is not part of any
crate, CI does not build or test it, and it makes no constant-time or
security claim. Findings are in [REPORT_SEEDING.md](REPORT_SEEDING.md).

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
(randomizable keys). **ePrint 2023/807 was read from indexed excerpts
because the PDF was blocked (Cloudflare) from the box. Algorithm numbers,
section citations, and the model of Alg. 10 must be checked against the
paper itself before anything here is cited.**

## Review notes (Rick, 2026-11-21)

These experiments are worth keeping as a measuring tool for the hardening
stage, not as the design of a LatticePrism type now.

- The harness leaves out PRISM's 32- to 96-byte salt, which is carried in
  the signature. Our challenge is computed from the key, that salt, and
  the message, so it is not known before signing, contrary to the report
  baselines that form the challenge from the public key and message alone.
- The "commitment" construction is essentially that existing salt, chosen
  from the secret key and the message. Shekyl already has that property
  with no new signature bytes.
- The challenge is public anyway. Timing that depends on it leaks nothing
  secret. What needs constant time is the work on the secret ideal and the
  norm equations, and those are modelled only roughly here.
- The model assumes a retry-heavy integer-representation step. Stage 1a
  found that our signing path has no such step.
- Three of the constructions here (COM, EPH, REV) change the signature
  format and how the challenge is hashed. That is a different scheme, with
  no security proof, and the replay test against the C reference would no
  longer apply. None of that should be adopted without a design round.

Intended later use: from stage 2 on, point the harness at the
secret-touching steps listed in each specification section, using our
constants and real secret ideals. In stage 7, use it as the first filter:
any secret-touching step whose count varies needs a fixed schedule, an
argument that its count cannot depend on the key, or blinding. Then port
the instrumentation to the Rust code, because step counts are not cycles.
