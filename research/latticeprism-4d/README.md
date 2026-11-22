# research/latticeprism-4d

**This is a research experiment for Shekyl PRISM, Rick Dawson's isogeny
signature scheme. Its code, data and conclusions, including negative ones,
are public by design, so that anyone working on constant-time isogeny or
lattice techniques has the same information we do.** It is not part of any
crate, CI does not build or test it, and it makes no constant-time or
security claim. Findings are in [REPORT_4D.md](REPORT_4D.md).

## Purpose

This tests the design under consideration for LatticePrism, a proposed
Rust type that maps PRISM targets (primes) to lattice coordinates for
constant-time prime derivation:

- **Prime-set tags.** The design names P363, P384, P500 and P664. This
  repository defines P248, P324, P376, P500 and P664 (`prism-field`), and
  those five are what was run. P363 and P384 are not repository parameter
  sets.
- **Four-dimensional embedding** with coordinates from a hash expansion
  (SHAKE-256 or BLAKE3 XOF of seed, target and domain tag), instead of a
  pure deterministic map.
- **Seed = H(public key).** There is no pre-shared secret.
- **Constant time that comes naturally** from the algorithm, without padding.

The 2-D experiment in [`../cornacchia-harness/`](../cornacchia-harness/)
(classical Cornacchia versus ePrint 2023/807 Alg. 10) is the negative
control.

Short result: hash-masking the basis of a fixed lattice (H1) keeps a
strong target dependence, as in 2-D. Hash-choosing a fresh lattice
instance (H2) flattens it, except for target size (covolume). A public
seed hides nothing from someone who knows the target. Only a fixed
schedule gives a constant trace. Read the structural flags in REPORT_4D §5
before building on any of this.

## Running

The Sage setup is the same as in `../cornacchia-harness/README.md`, and
`blake3` was added with pip.

```sh
micromamba run -n sage python run_4d.py --n 600 --tours 300 --panel 24 --draws 150  # ~18 min, 7 procs
micromamba run -n sage python run_secret.py --keys 20 --draws 100                  # ~8 min
micromamba run -n sage python analyze_4d.py
micromamba run -n sage python h1_modes.py
micromamba run -n sage sage crosscheck_4d.sage
```

## Reference papers

- Jacquemin, Mukherjee, Kutas, Sinha Roy, *Ready to SQI? Safety First!*,
  ePrint 2023/807 (<https://eprint.iacr.org/2023/807>). Its Cornacchia
  leak and randomized-basis variant are the 2-D control.
- Hanyecz, Karenin, Kirshanova, Kutas, Schaeffler, *Constant time lattice
  reduction in dimension 4 with application to SQIsign*, TCHES 2025. It
  gives a constant-time dimension-4 LLL with shortest-vector guarantees.
  Artifact: <https://artifacts.iacr.org/tches/2025/a13/>; code:
  <https://github.com/CTlll-SQIsign/CTlll-SQIsign>.
- Basso, He, Jacquemin, Kouider, Kutas, Mukherjee, Schaeffler, Sinha Roy,
  *Constant-time Quaternion Algorithms for SQIsign*, ePrint 2025/2192
  (<https://eprint.iacr.org/2025/2192>). It gives a constant-time HNF,
  ideal generator and GeneralizedRepresentInteger.
- *Isogeny-based Signatures with Randomizable Keys*, ePrint 2026/1169
  (<https://eprint.iacr.org/2026/1169>). It covers the randomizable-key
  variants SQIsign-RK, PRISM-RK and DeuringVUF-RK.

The follow-up that compares seed constructions for H2 (secret
deterministic seeds, commitments, ephemeral keys, and a reverse map) is
in [`../latticeprism-seeding/`](../latticeprism-seeding/).
