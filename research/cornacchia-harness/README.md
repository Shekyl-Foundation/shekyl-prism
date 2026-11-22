# research/cornacchia-harness

An exploratory harness that compares the execution traces of classical
Cornacchia with the randomized-basis lattice variant of ePrint 2023/807
(Alg. 10) on prime streams at PRISM-sized inputs. It is research only. It
is not part of any crate, is not built or tested by CI, and makes no
constant-time claim. Findings are in [REPORT.md](REPORT.md).

## Why this exists: LatticePrism

LatticePrism is a proposed Rust type that maps primes to lattice coordinates
so that PRISM's isogeny prime derivation can be constant time. Early notes
for it used the tags P363, P384, P500 and P664. P363 and P384 are **not**
Shekyl parameter sets. The sets in this repository, defined in `prism-field`,
are P248, P324, P376, P500 and P664:

- The map is seeded from a hash of the public key. There is no pre-shared
  secret, because this is public-key cryptography.
- The goal is primitives where constant time is a natural property of the
  algorithm, not something added with padding or rigid dummy loops.

This harness asks whether the 2023/807 trick (mask the 2-D lattice basis with
a random unimodular matrix, then Lagrange-reduce) gives that property. In this
model it does not. The masked trace equals the unmasked lattice trace plus
independent noise (REPORT §3), and a mask derived from a public seed would
hide nothing at all.

The harness ran at bit lengths 376, 384, 500 and 664. 376, 500 and 664
match the Shekyl sets P376, P500 and P664; 384 is a bit length only, not a
Shekyl parameter set.

## Running

The harness needs SageMath 10.x (for the cross-check) or just Python 3.12 with
gmpy2, numpy, scipy, pandas and matplotlib. The setup used:

```sh
# micromamba, no root needed
micromamba create -n sage -c conda-forge sage python=3.12 pandas
micromamba run -n sage python run_experiments.py --sizes 376 384 500 664 \
    --n 5000 --nrandom 100000 --fixed 30 --draws 1000 --out results  # ~2 min
micromamba run -n sage python analyze.py          # summary.json, plots/
micromamba run -n sage python s_scaling.py        # mask-size experiment
micromamba run -n sage sage sage_crosscheck.sage  # vs two_squares / qfbcornacchia
micromamba run -n sage gp -q pari_crosscheck.gp
```

Tested with Sage 10.10, PARI 2.17.4, gmpy2 2.3.2, numpy 2.5.5, scipy 1.18.1
and pandas 3.1.1.

## Reference papers

- Jacquemin, Mukherjee, Kutas, Sinha Roy, *Ready to SQI? Safety First!
  Towards a constant-time implementation of isogeny-based signature,
  SQIsign*, ePrint 2023/807, <https://eprint.iacr.org/2023/807>. It shows a
  Cornacchia input leak recovers the SQIsign key, and proposes a CT half-GCD
  and the randomized-basis lattice variant (Alg. 10) modelled here. **This
  paper was read from indexed excerpts because the PDF was blocked
  (Cloudflare) from the box. Algorithm numbers, section citations, and the
  model of Alg. 10 must be checked against the paper itself before anything
  here is cited.**
- Hanyecz, Karenin, Kirshanova, Kutas, Schaeffler, *Constant time lattice
  reduction in dimension 4 with application to SQIsign*, TCHES 2025. It gives
  a constant-time LLL-like reduction of 4-dimensional integral lattices, with
  shortest-vector guarantees, verified constant time with tooling. Artifact:
  <https://artifacts.iacr.org/tches/2025/a13/>; code:
  <https://github.com/CTlll-SQIsign/CTlll-SQIsign>.
- Basso, He, Jacquemin, Kouider, Kutas, Mukherjee, Schaeffler, Sinha Roy,
  *Constant-time Quaternion Algorithms for SQIsign*, ePrint 2025/2192,
  <https://eprint.iacr.org/2025/2192>. It gives a constant-time HNF, ideal
  generator and GeneralizedRepresentInteger for the quaternion layer.
- *Isogeny-based Signatures with Randomizable Keys*, ePrint 2026/1169,
  <https://eprint.iacr.org/2026/1169>. It covers randomizable-key variants
  SQIsign-RK, PRISM-RK and DeuringVUF-RK.

## Transparency, and the follow-up

This is a research experiment for Shekyl PRISM. PRISM is the signature
scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin, Wouter Castryck,
Maria Corte-Real Santos, Riccardo Invernizzi, Antonin Leroux, Luciano
Maino, Frederik Vercauteren, and Benjamin Wesolowski; the paper is at
`docs/papers/2026-443_PRISM_with _salt.pdf`). Shekyl's `shekyl-prism` is
an implementation of it. The experiment's results are public by design,
negative ones included. The 4-D hash-expanded follow-up, which treats this
experiment as its negative control, is in
[`../latticeprism-4d/`](../latticeprism-4d/).

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
- Three of the later seed constructions change the signature format and
  how the challenge is hashed. That is a different scheme, with no
  security proof, and the replay test against the C reference would no
  longer apply. None of that should be adopted without a design round.

Intended later use: from stage 2 on, point the harness at the
secret-touching steps listed in each specification section, using our
constants and real secret ideals. In stage 7, use it as the first filter:
any secret-touching step whose count varies needs a fixed schedule, an
argument that its count cannot depend on the key, or blinding. Then port
the instrumentation to the Rust code, because step counts are not cycles.
