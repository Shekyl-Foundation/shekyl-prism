# Proposed CI change (not applied to `dev`)

This folder is a proposal only. It is not a change to
`.github/workflows` on `dev`, and it must not be treated as one
when this experiment is folded into the `research` branch.

## Recommendation

Add certificate verification to the existing `differential-prime`
nightly job, and also to ordinary CI, **in addition to** the current
SymPy `nextprime` check. Do not delete
`scripts/ci/check_signer_primes.py` in the first adoption PR.

A primality certificate proves that a number is prime. The SymPy
step also checks that the secret norm is the *next* prime after
`2^(2·WIDTH_BITS)`. The new verifier already proves that gap claim
directly (and does the same for the item-19 cofactors, which SymPy
does not currently check). Until the certificates live on `dev` and
the verifier has been reviewed, keeping SymPy is cheap defence in
depth. A later PR can drop the SymPy step and close ledger O19.

The new step needs only Python 3. No SymPy, no PARI, no Rust
toolchain, and no `gmpy2`. It takes about five seconds.

## Nightly snippet

Apply the patch in `nightly-differential-prime.diff` to
`.github/workflows/nightly.yml` on a future `dev` PR, after the
certificate files have been moved to a path that `dev` ships (or
after `research/prime-certificates/` is accepted as the temporary
home, which is not recommended for production CI).

The intended command, once the verifier lives at a production path,
is:

```
python3 scripts/ci/check_signer_prime_certificates.py
```

That wrapper, also only a proposal, is
`check_signer_prime_certificates.py` in this folder. Until a move is
agreed, the equivalent command in this experiment is:

```
python3 research/prime-certificates/verify.py
```

A self-test of the arithmetic, useful in the same job or as a
pre-commit, is:

```
python3 research/prime-certificates/verify.py --selftest
```

Do not require `verify.py --pari` in CI. That needs PARI and is only
a cross-check.

## Main CI

The same `verify.py` / wrapper command is short enough for
`.github/workflows/ci.yml`. A natural place is alongside the other
constants-integrity checks, after checkout, with no extra toolchain
setup. The exact insertion point should be chosen in the adoption
PR so that it does not sit behind the Rust build.

## What this does not do

It does not change any generated constant, the signer, or the
parameter sets. If a future regeneration of `prism-vectors/data/signer/`
moves a secret norm or a cofactor, the verifier will fail until new
certificates are pinned.
