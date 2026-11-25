# Primality certificates for the signer's fixed primes

Research harness for Shekyl PRISM. PRISM is the signature scheme of
ePrint 2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi,
Leroux, Maino, Vercauteren, and Wesolowski); `shekyl-prism` implements
it. This directory does not change the signer, the signature format, or
any parameter set.

Sets covered: P248, P324, P376, P500, and P664 only.

## Question

The constants generator and the nightly SymPy check treat the field
moduli and the secret-ideal starting norms as Baillie–PSW probable
primes (ledger O19). For five fixed constants that is a regression
test, not a primality certificate. Can we pin independently checkable
certificates for every fixed prime the generator relies on, and
replace perpetual probable-prime retesting with a short, standard-
library verifier?

## What is certified

The inventory is in [`PRIMES.md`](PRIMES.md) and [`primes.json`](primes.json).
In short:

- Five field primes `p = f · 2^e − 1` (251–669 bits).
- Four unique secret-ideal starting norms, item 33 (513–1537 bits).
  P324 and P376 share one value, because they share `WIDTH_BITS = 384`.
- Five prime cofactors, item 19 (252–670 bits). These are used only by
  two C-reference benchmarks, but the generator still relies on them.
- The 101 `small_primes` and the 102 complementary primes that are
  `3 (mod 4)` below 1237, by trial division.

Runtime probable primes (challenge degrees and the like) are not
fixed and are not certified here.

## Methods

Every large prime has an Atkin–Morain ECPP certificate produced by
PARI/GP 2.17.4 `primecert` (flag 0). The same proof is also exported
in Primo format 4; that is a re-encoding, not a second method.

Each field prime has a second, independent Lucas `N+1` certificate.
This is the short proof O19 already pointed at: `p + 1 = f · 2^e` is
completely factored.

For every “next prime after `2^k`” (the secret norms and the
cofactors), the verifier also proves that every integer strictly
between `2^k` and the claimed prime is composite, by a factor or an
unconditional Fermat / Miller–Rabin witness. A primality certificate
alone would not prove the “next” part of item 19 or item 33.

The ECPP leaf (a prime `≤ 2^64`) is proved by Pratt’s theorem after a
complete factorisation of `q − 1`, or by trial division when
`q ≤ 2^20`. The verifier never calls `primecertisvalid` for its
verdict. `verify.py --pari` is an optional cross-check only.

## How to re-check

From the repository root, Python 3.8+ and the standard library are
enough. `gmpy2` is used when it is importable, only for speed.

```
python3 research/prime-certificates/verify.py --selftest
python3 research/prime-certificates/verify.py
```

On this box both commands finish in about five seconds. The second
command holds the inventory equal to the committed signer constants,
proves the tiny lists, proves the next-prime gaps, and verifies every
pinned ECPP and Lucas certificate.

To also ask PARI whether it accepts its own ECPP files (not required):

```
python3 research/prime-certificates/verify.py --pari
```

To rebuild the certificates (needs PARI 2.17 with the `cypari2`
bindings, as in the micromamba Sage environment at `/workspace/mamba`):

```
python3 research/prime-certificates/generate.py
```

The CI-ready command is the same `verify.py` invocation. A proposed
nightly change, not applied to `dev`, is in [`proposed-ci/`](proposed-ci/).

## Layout

- `primes.json`, `PRIMES.md` — the inventory.
- `verify.py` — the independent verifier.
- `generate.py` — how the certificates were produced.
- `certs/` — pinned certificates and `manifest.json` (timings and
  SHA-256 hashes).
- `REPORT.md` — results and how this would slot into `dev`.
- `proposed-ci/` — a clearly labelled proposal for nightly/CI, not a
  change to `.github/workflows` on this branch.
