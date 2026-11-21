# research/cornacchia-harness

An exploratory harness that compares the execution traces of classical
Cornacchia with the randomized-basis lattice variant of ePrint 2023/807
(Alg. 10) on prime streams at PRISM-sized inputs. It is research only. It
is not part of any crate, is not built or tested by CI, and makes no
constant-time claim. Findings are in [REPORT.md](REPORT.md).

## Why this exists: LatticePrism

LatticePrism is a proposed Rust type that maps primes to lattice coordinates
so that PRISM's isogeny prime derivation can be constant time. It is
parameterized by a prime-set tag (P363, P384, P500, P664):

- The map is seeded from a hash of the public key. There is no pre-shared
  secret, because this is public-key cryptography.
- The goal is primitives where constant time is a natural property of the
  algorithm, not something added with padding or rigid dummy loops.

This harness asks whether the 2023/807 trick (mask the 2-D lattice basis with
a random unimodular matrix, then Lagrange-reduce) gives that property. In this
model it does not. The masked trace equals the unmasked lattice trace plus
independent noise (REPORT §3), and a mask derived from a public seed would
hide nothing at all.

The tags P363 and P384 are LatticePrism names. They are **not** parameter
sets in this repository, which defines P248, P324, P376, P500 and P664 in
`prism-field`. The harness ran at 376, 384, 500 and 664 bits; 384 stands in
for the proposed P384.

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
  and the randomized-basis lattice variant (Alg. 10) modelled here.
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
