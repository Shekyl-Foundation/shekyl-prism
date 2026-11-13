# Security review handoff

Status on 2026-11-12: executable research signer/verifier, not approved for
wallet authorization or consensus. Correct execution and security proofs
are separate requirements. No external reviewer has approved these changes.

The separate [code-matched security argument](SECURITY_ARGUMENT.md) pins
the salted implementation of PR #15, defines its SUF-CMA game and
conditional reduction, and maps the published salt-PRISM bound to the
actual parameters. The [salt comparison](SALT_COMPARISON.md) records the
measurements with and without the salt. The argument's review status is
DRAFT; the obligations below remain open.
Reproduce its integer bounds with `python3 scripts/security_parameters.py`.

## Implemented and checked

The Rust verifier executes two adapted theta chains, compares dual codomains
and both components of all three image points. The old always-refusing path
is no longer the public path. It enforces unique field bytes, joint sign,
least Montgomery model and excludes extra-automorphism response classes.
Invalid all-zero projective vectors cannot compare equal to valid points.

113 honest fixtures pass. The intermediate nulls of both chains match the
six primary Sage traces. Malleability regressions and generated native
signatures exercise the complete acceptance relation. The native signer
uses a SHAKE256 stream instead of the fixture RNG and derives the signing
stream from the secret master seed and challenge scalar. Thus the same
key and degree produce the same response, including under hash collisions.
The challenge hash binds a carried salt of 32/48/64 bytes; the salt comes
from a secret-keyed schedule over the message, a 32-byte caller input and
a counter that is never output.

The [conditional acceptance argument](ACCEPTANCE_ARGUMENT.md) derives
consistent signs and prime degree from the polarized-chain premise. That
premise and the protocol reduction still require independent review.

## Review items that remain open

1. **Exact acceptance theorem.** Prove that the adapted bases, dual
   identification and the two component comparisons on `P,Q,P−Q` establish
   the intended prime-degree isogeny and a unique normalized encoding. In
   particular, justify the residual per-factor signs. Empirical tests and
   a correct theta port do not establish this theorem.
2. **Parameter change.** The field rows were published for salt-PRISM.
   This prototype changes §5's exponent to `e′=e−2`. Verify the torsion,
   norm and uniqueness hypotheses for the actual available evaluation
   subgroup, not an assumed extra two powers of rational torsion.
3. **Challenge entropy and second preimages.** Odd `s` has `e−4` free
   bits: 244, 372, 496. With the salt, a third party's hash-side attack
   is a second preimage onto a signed scalar, about `2^b/N_sign` hash
   queries in the random-oracle estimate, against `sqrt(2^b/ρ)` for the
   unsalted collision replay (`2^125.3`, `2^189.6`, `2^251.7` at the
   measured densities 1.055%, 0.695%, 0.584%). Both attacks were run
   against the real signer and verifier at projection widths 16 to 32
   only; the full-width figures are extrapolations. The cost of memory,
   one hash, and quantum attacks needs explicit accounting. A
   491/747/995-bit degree is not itself a challenge-entropy argument. Do
   not claim the paper's security levels: its Theorem 2 is mapped in the
   security argument §6 together with what does not carry over.
4. **Salt schedule and retry policy.** The salt must be unpredictable
   before signing. That rests on a PRF assumption for cSHAKE256 keyed by
   the secret `salt_key`, or by fresh caller randomness when the hedge is
   relied on; neither is proved, and a leaked or attacker-chosen salt
   returns the unsalted collision cost. The signer skips a prime candidate
   on a backend error, malformed response or refused response. Give the
   distribution of the released scalar under that policy, which the
   published reduction needs uniform over prime degrees. The counter is
   not in the signature, but signing time depends on it. A signer can
   still issue one signature valid for two messages of its own choosing
   at the unsalted collision cost.
5. **Primality.** BPSW is a reproducible probable-prime predicate. No bypass
   was found, but this is not a certificate or a proof against adversarial
   grinding. Decide on a justified deterministic test policy or a bounded,
   canonical signer-supplied certificate and account for the wire cost.
6. **Signer side channels and secret lifecycle.** Audit native keygen,
   quaternion sampling, ideal-to-isogeny, failure distributions and their
   memory access. The Rust seed buffers use zeroize, backend RNG/input
   buffers are cleared, native cores are disabled, scratch is mode 0700,
   key files are created exclusively as 0600, and normal exits remove
   scratch. This is incomplete memory hardening: SHAKE internal states in
   Rust, native allocations, abnormal-exit scratch and storage remnants
   require review. The implementation is not claimed constant time.
7. **Key distribution and rerandomization.** Specify the per-output secret
   derivation and prove its relation to the public key and unknown
   endomorphism ring. Neither wallet integration nor key rerandomization
   is implemented. ECLIPSE's salted construction and its stated future
   SUF-CMA work do not provide a proof for this interface.
8. **Deployment costs.** Successful verification has been measured on the
   Pi 4 Model B: the median of three level III run medians is 449.076 ms
   under the recorded conditions; see [BENCHMARKING.md](BENCHMARKING.md).
   A verification budget, adversarial-cost measurements and resource limits
   remain deployment decisions. This is not an idle-device latency bound.

## How to review reproducibly

Run workspace debug and release tests, then the native signing test against
all three patched backends. Install Sage, initialize the exact pinned
Theta_dim4 submodule, build `s5_challenge` and `verify_bytes`, and run
`sage scripts/sage/differential.sage`. It compares both implementations on
the current honest corpus and mutations without rewriting known answers.
A matching result is evidence of implementation agreement, not a security
proof. `sh scripts/sage/run_container.sh <run-name>` runs it in the pinned
Sage image and keeps the log, exit status and input digests under
`target/sage/<run-name>/`.

**Ran** on the salted corpus at commit `2c2534f`: Sage and Rust returned
the same verdict on 485 signatures and mutations across all 113 vectors,
each vector in its own interpreter, and every honest signature was `Ok`
in both (exit status 0). The log, status and input digests are in
[comparison/sage-differential/](comparison/sage-differential/). The two
`Timeout` stack traces in the log are the script's periodic progress
dump on slow steps, not failures. Every row of `accept_verdicts.tsv` was
checked against that run and none changed.

The comparison experiments are reproduced with
`sh scripts/run_comparison.sh` and `python3 scripts/analyze_comparison.py`;
[SALT_COMPARISON.md](SALT_COMPARISON.md) lists the command of each.

Before adoption, require an independent review of the exact written
protocol, its parameter accounting and native signer, plus a deployment
budget evaluated against the recorded floor-device measurement. No notification to paper authors or other
reviewers has been sent.
