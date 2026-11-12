# Security review handoff

Status on 2026-11-12: executable research signer/verifier, not approved for
wallet authorization or consensus. Correct execution and security proofs
are separate requirements. No external reviewer has approved these changes.

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
3. **Challenge entropy and collision attacks.** Odd `s` has `e−4` free
   bits: 244, 372, 496. Raw scalar birthday scales are `2^122`, `2^186`,
   `2^248`. A usable replay attack also requires a prime-degree collision
   at indices the signer will use. For fixed first index and prime fraction
   `ρ`, the random-function estimate is `sqrt(2^(e−4)/ρ)` hash queries;
   the cost of filtering, memory and quantum attacks needs explicit
   accounting. A 491/747/995-bit degree is not itself a challenge-entropy
   argument. Do not claim the paper's security levels without resolving
   this. Salt binding or larger parameters would change the protocol and
   need new vectors and analysis.
4. **Carried index.** The verifier accepts any prime index, not a certified
   hash-to-prime output. Give a SUF-CMA argument for that interface and
   the deterministic failure/retry policy. The signer is deterministic
   for a degree, but that does not by itself repair a reduction.
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
   SUF-CMA work do not provide a proof for this unsalted interface.
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
proof. The fresh Sage comparison was not run in this environment.

Before adoption, require an independent review of the exact written
protocol, its parameter accounting and native signer, plus a deployment
budget evaluated against the recorded floor-device measurement. No notification to paper authors or other
reviewers has been sent.
