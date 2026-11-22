# research/latticeprism-4d

**This is a research experiment for Shekyl PRISM. PRISM is the signature
scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin, Wouter Castryck,
Maria Corte-Real Santos, Riccardo Invernizzi, Antonin Leroux, Luciano
Maino, Frederik Vercauteren, and Benjamin Wesolowski; the paper is at
`docs/papers/2026-443_PRISM_with _salt.pdf`). Shekyl's `shekyl-prism` is
an implementation of it. The experiment's code, data and conclusions,
including negative ones, are public by design, so that anyone working on
constant-time isogeny or lattice techniques has the same information we
do.** It is not part of any crate, CI does not build or test it, and it
makes no constant-time or security claim. Findings are in
[REPORT_4D.md](REPORT_4D.md).

## Purpose

This tests the design under consideration for LatticePrism, a proposed
Rust type that maps PRISM targets (primes) to lattice coordinates for
constant-time prime derivation:

- **Prime-set tags.** Early LatticePrism notes named P363, P384, P500 and
  P664. P363 and P384 are **not** Shekyl parameter sets. This repository
  defines P248, P324, P376, P500 and P664 (`prism-field`), and those five
  are what was run.
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
  leak and randomized-basis variant are the 2-D control. **This paper was
  read from indexed excerpts because the PDF was blocked (Cloudflare) from
  the box. Algorithm numbers, section citations, and the model of Alg. 10
  must be checked against the paper itself before anything here is cited.**
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
