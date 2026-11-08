# shekyl-prism

Research workspace for an isogeny engine and a spec-faithful PRISM verifier.
It is not a consensus crate, it does not sign, and it is not wired to a
transaction.

The gap it exists for is in the wallet derivation: each output's ML-DSA seed
is expanded from the combined shared secret, which the sender computed. A
later design round can consider PRISM as a third mandatory authorization leg
only after the level III verification cost has been measured on the floor
device (Raspberry Pi 4 Model B). Until that number exists, this repository
does not propose that leg.

## What is here

| Crate | Role |
| --- | --- |
| `prism-field` | The three Table 2 primes, on `fp2` 0.6.0 with default features off |
| `prism-isogeny` | Montgomery curves, Vélu 2-isogenies, and split (2,2) chains |
| `prism-verify` | Pure decoder and verifier. Bad input is a typed refusal |
| `prism-bench` | Times NIST I, III, and V on one machine |

The verifier spec is [`docs/VERIFIER_SPEC.md`](docs/VERIFIER_SPEC.md). It is a
draft to take to the authors, not a standard. Working copies of the papers
are in the private `shekyl-dev` repository under `docs/papers/`.

The C reference [PRISM_v2](https://github.com/mariascrs/PRISM_v2) is the
submodule `external/PRISM_v2`, pinned, and used only by
`scripts/check_c_reference.py`. The Rust crates do not link it. That tree
implements salt-PRISM. Its verifier resamples a salt and uses a
probabilistic prime test. This workspace does not.

## Field backend

`fp2`'s packed arithmetic is off. The level III prime `65·2^376 − 1` is 383
bits, so it sits in six 64-bit limbs and is not strictly below `2^382`. The
feature's own modulus rule therefore does not hold for every prime, and one
crate feature cannot enable it for level I and level V while leaving level
III on the scalar path. The bench reports `field-backend=scalar`.

## Build

```bash
cargo test --workspace
cargo run -p prism-bench --release
python3 scripts/check_c_reference.py
```

`cargo run -p prism-bench` prints wall time and the userspace counter
(`cntvct_el0` on aarch64, `rdtsc` on x86_64). The synthetic input's challenge
degree is not prime: `outcome=composite` means the chain of length `a`
finished and Baillie–PSW refused `q`. That line is not an acceptance. The
consensus gate is the level III line on the floor device. Levels I and V are
the same run, so the cost curve is measured. A number from any other machine
is not that gate.
