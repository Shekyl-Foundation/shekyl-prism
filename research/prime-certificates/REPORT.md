# Report: pinned primality certificates (ledger O19)

Research report for Shekyl PRISM. This directory does not change the
signer. Sets covered: P248, P324, P376, P500, and P664 only.

Base: `dev` at `26ce1410286728e21120fc98da710b6995a04920`.
Certificates generated on 24 November 2026, 23:08 America/New_York,
with PARI/GP 2.17.4 (two threads) in the Sage micromamba environment.

## Verdict

Every fixed prime the constants generator relies on now has a pinned,
independently checkable primality proof. The nightly SymPy
`nextprime` check can stay as a regression on the “next after `2^k`”
rule, but it is no longer the only evidence, and it is not a proof.
A five-second standard-library verifier replaces perpetual
probable-prime retesting for these constants.

This is contained work. It closes the mathematical assumption in
ledger O19 without changing the signing protocol, the secret-key
format, or any parameter set.

## What was certified

Fourteen unique large primes, plus two tiny lists proved by trial
division. P324 and P376 share the secret norm `nextprime(2^768)`, so
there are five secret-norm listings and four unique values.

| ID | Sets | Formula | Bits | Methods |
|----|------|---------|------|---------|
| field-p248 | P248 | `5·2^248 − 1` | 251 | ECPP (10 steps) and Lucas N+1 (`P=5`, `Q=−1`) |
| field-p324 | P324 | `3·2^324 − 1` | 326 | ECPP (11 steps) and Lucas N+1 (`P=1`, `Q=−1`) |
| field-p376 | P376 | `65·2^376 − 1` | 383 | ECPP (10 steps) and Lucas N+1 (`P=5`, `Q=−1`) |
| field-p500 | P500 | `27·2^500 − 1` | 505 | ECPP (26 steps) and Lucas N+1 (`P=5`, `Q=−1`) |
| field-p664 | P664 | `17·2^664 − 1` | 669 | ECPP (22 steps) and Lucas N+1 (`P=7`, `Q=−1`) |
| secret-norm-2p512 | P248 | `2^512 + 75` | 513 | ECPP (16 steps); gap of 74 composites proved |
| secret-norm-2p768 | P324, P376 | `2^768 + 183` | 769 | ECPP (25 steps); gap of 182 composites proved |
| secret-norm-2p1024 | P500 | `2^1024 + 643` | 1025 | ECPP (37 steps); gap of 642 composites proved |
| secret-norm-2p1536 | P664 | `2^1536 + 75` | 1537 | ECPP (60 steps); gap of 74 composites proved |
| cofactor-p248 | P248 | `2^251 + 65` | 252 | ECPP (8 steps); gap of 64 composites proved |
| cofactor-p324 | P324 | `2^326 + 249` | 327 | ECPP (11 steps); gap of 248 composites proved |
| cofactor-p376 | P376 | `2^383 + 369` | 384 | ECPP (12 steps); gap of 368 composites proved |
| cofactor-p500 | P500 | `2^505 + 51` | 506 | ECPP (22 steps); gap of 50 composites proved |
| cofactor-p664 | P664 | `2^669 + 1067` | 670 | ECPP (28 steps); gap of 1066 composites proved |

The 101 `small_primes` (2 through 1237) and the 102 complementary
primes that are `3 (mod 4)` below 1237 are proved by trial division
and checked against `shared.txt`. Decimal values and SHA-256 hashes
of those values are in `primes.json`. Per-file hashes of the
certificates are in `certs/manifest.json`.

## Timings

ECPP generation, wall time on this box (PARI 2.17.4, two threads):

| Prime | Seconds | Steps |
|-------|---------|-------|
| field-p248 | 0.047 | 10 |
| field-p324 | 0.135 | 11 |
| field-p376 | 0.064 | 10 |
| field-p500 | 0.355 | 26 |
| field-p664 | 0.398 | 22 |
| secret-norm-2p512 | 0.185 | 16 |
| secret-norm-2p768 | 0.443 | 25 |
| secret-norm-2p1024 | 0.941 | 37 |
| secret-norm-2p1536 | 3.469 | 60 |
| cofactor-p248 | 0.036 | 8 |
| cofactor-p324 | 0.056 | 11 |
| cofactor-p376 | 0.050 | 12 |
| cofactor-p500 | 0.193 | 22 |
| cofactor-p664 | 0.360 | 28 |
| **total ECPP** | **6.73** | |

Lucas `N+1` search and check for the five field primes together took
about 20 milliseconds. Independent verification of the whole tree,
including next-prime gaps and the tiny lists, takes about 4.5–5.2
seconds with CPython 3.12 plus gmpy2 2.3.2, and about the same with
CPython 3.13 and no gmpy2. `verify.py --pari` agrees with
`primecertisvalid` on all fourteen ECPP files; that cross-check is
not used for the independent verdict.

## Verifier results

- `python3 research/prime-certificates/verify.py --selftest` passed
  on both the gmpy2 and the pure-Python backends.
- `python3 research/prime-certificates/verify.py` accepted all
  fourteen large primes, held every inventory value equal to the
  committed signer data, proved both tiny lists complete, and proved
  all nine next-prime gaps.
- `python3 research/prime-certificates/verify.py --pari` then agreed
  with PARI on every `.ecpp.gp` file.

The ECPP checker implements the conditions in the PARI 2.17 manual
(`usersch3`, `primecert`): `t^2 < 4N`, `s` divides `m = N+1-t`,
`q = m/s > (N^{1/4}+1)^2` by an exact integer test, `mP = ∞` and
`sP ≠ ∞` on `y^2 = x^3 + ax + b` in Jacobian coordinates over
`Z/NZ`, and a Pratt (or trial) proof that the final `q ≤ 2^64` is
prime. The Lucas `N+1` checker uses the complete factorisation of
`p+1` and the standard `U_{N+1}` conditions.

A Primo format 4 file is stored beside each ECPP certificate as a
convenience for tools that read Primo certificates. It encodes the
same Atkin–Morain proof, so it is not counted as a second method.

## What was not feasible as a second method

A PARI `N−1` (Pocklington–Lehmer) certificate was not used as the
second method. For the field primes it is the wrong tool: `N−1` is
not the completely factored side, `N+1` is. For the secret norms and
cofactors, `N−1` has a large unfactored odd part (typically only a
few small prime factors fall out of a 10,000 trial), so a general
`N−1` search is the “very slow” path the PARI manual warns about.
ECPP finished in seconds for every such prime, including the
1537-bit secret norm, so ECPP is the practical certificate there.
The next-prime gap proofs supply the missing “next after `2^k`”
half of items 19 and 33, which an `N−1` certificate would not have
given by itself.

Item 19 (the prime cofactor) is certified even though it is not on
the native signing path, because the generator still relies on the
rule “next prime after `2^{bit_length(p)}`”.

## How this would slot into `dev`

Nothing here has to land before stages 2b–2d. It is optional
hardening of stage 1, and it is small enough to take whenever the
certificates and verifier are reviewed.

A suggested adoption path, for a later design-round PR, not done on
this branch:

1. Move `certs/`, `verify.py`, and `primes.json` to a production
   path (for example `prism-vectors/data/signer/certificates/` and
   `scripts/ci/check_signer_prime_certificates.py`).
2. Add the five-second `verify.py` command to the
   `differential-prime` nightly job, and to ordinary CI: it needs
   only Python, no SymPy, no PARI, no Rust toolchain.
3. Keep `scripts/ci/check_signer_primes.py` for a while as a
   cross-check that SymPy’s `nextprime` still names the same secret
   norms. The gap proofs in the new verifier already prove that
   claim, more strongly, for both the secret norms and the
   cofactors, so the SymPy step can later be dropped.
4. Close ledger O19 once the certificates and the verifier live on
   `dev` and the nightly runs them.

The proposed patch is in `proposed-ci/`. It is not applied to
`.github/workflows` on this experiment branch, and it must not be
merged to `dev` with the research fold.

A primality proof does not change any generated constant. If a
future constants regeneration produced a different secret norm or
cofactor, verification would fail until new certificates were
pinned; that is the intended behaviour.

## Command record

```
python3 research/prime-certificates/verify.py --selftest
python3 research/prime-certificates/verify.py
python3 research/prime-certificates/verify.py --pari
```

All three passed on this box after the certificates were generated
by `python3 research/prime-certificates/generate.py --nm1-timeout 0`.
