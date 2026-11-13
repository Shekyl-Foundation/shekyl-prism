# Research verifier specification

Status: salted executable prototype, 2026-11-12. This is a local instantiation of
PRISM's prime-degree §5 relation with a salted challenge, not an adopted standard or a reviewed
SUF-CMA security claim. See [security review](SECURITY_REVIEW.md).
The [code-matched security argument draft](SECURITY_ARGUMENT.md) defines
the exact attacker game, parameter audit and conditional proof obligations.
The [earlier design notes](archive/VERIFIER_SPEC_before_acceptance_2026-11-12.md)
preserve the derivations, investigations and questions that led here.

## Parameters and encoding

| Parameter | I | III | V |
| --- | --- | --- | --- |
| `p = f·2^e − 1` | `5·2^248−1` | `65·2^376−1` | `27·2^500−1` |
| Rational torsion exponent `e` | 248 | 376 | 500 |
| Long chain `e′ = e−2` | 246 | 374 | 498 |
| Short chain `e′−1` | 245 | 373 | 497 |
| Challenge width `e′−1` | 245 | 373 | 497 |
| Free bits in odd `s` | 244 | 372 | 496 |
| Degree `q` bit length | 491 | 747 | 995 |
| Public-key bytes | 64 | 96 | 128 |
| Salt bytes (2λ bits) | 32 | 48 | 64 |
| Signature bytes | 352 | 528 | 704 |

I/III/V identify the paper's field rows. They do not certify the security
level of this modified salted protocol. `CHAIN_STEPS` is `e′`; `E` is
always the rational torsion exponent. `CHALLENGE_BITS` is the actual
`e−3` width, including the forced odd bit. The historical `e−1` constant
had the same byte width but overstated the number of challenge bits.

A public key is the canonical little-endian fp2 encoding of the Montgomery
coefficient `A`, for `y²=x³+Ax²+x`. The first component is real, the second
imaginary, each an integer in `[0,p)`. Singular keys and `A=0` are refused.
The accepted key bytes are its identity for the challenge hash.

The signature is `A_sig || Px || Py || Qx || Qy || salt`, five fp2
encodings and exactly the row's salt length. Field components are not reduced
on decoding. Every salt byte string of that length is admissible: salt
binding comes from the hash, not a unique numerical salt representation.
Truncation, extension and out-of-range field elements are refused. The
archived two-byte-index encoding and the paper's C wire encoding are not
accepted by default verification.

## Challenge and signing

`s = cSHAKE256(S="shekyl-prism-challenge-v2";
A_bytes || salt || message)`, truncated to `e−3` bits with bit zero set.
The leading key and salt lengths are fixed for the row. `q=2^(2e−5)−s²`.
Public verification tests that single degree and never searches salts.

The degree test remains deterministic Baillie–PSW: small-prime sieve,
perfect square rejection, strong base-2 Miller–Rabin and strong
Lucas–Selfridge. It is a probable-prime predicate, not a certificate;
adversarial adequacy remains unresolved.

The [salt protocol contract](SALT_PROTOCOL.md) specifies the 32-byte caller
randomness input and the secret-keyed cSHAKE salt schedule. Research signing
uses zeros by default; a caller may provide fresh randomness without
changing verification. The internal counter is never encoded. Signing
visits at most 16 prime candidates and 65536 counters, skips candidate
backend errors, malformed responses and failed final verification, and
returns only exact bytes accepted by Rust. Initial setup/keygen errors
remain typed errors. Exhaustion returns no signature. The response stream
remains deterministic for each key and scalar. The PRF, rejection/failure
distribution, and stronger isogeny assumptions are still review obligations.

## Normalization

The signature must satisfy `0 < y(P_sig) < y(−P_sig)` in the fp2 order:
compare the real components as integers first, then the imaginary ones.
The signer negates both points together when necessary. This follows
Algorithm 2's convention, correcting the opposite inequality printed in
Algorithm 3 line 1.

`A_sig²=4` is refused as Singular. The response coefficient must be the
least coefficient in its Montgomery isomorphism class, under the same fp2
order. The candidates are `±A` and `±(3α+A)/λ` for the two nonzero rational
2-torsion roots `α`, with `λ²=α²−1`. Full rational high-order torsion
ensures the required rational transports exist for an honest signature.
The exporter transports y using `λ^(3/2)` before joint normalization.

The response classes `j=0` (`A²=3`) and `j=1728` (`A=0` or `2A²=9`) are
excluded as Model. Choosing a coefficient alone cannot normalize their
extra automorphisms. This is an explicit restriction of the research
encoding; a signer skips such a response. No claim of support for those
classes is made.

## Canonical public-key basis

The bounded C-reference basis-index search is unchanged: search from 1
through `2λ`, including the C's seven-bit hint fallback in one search.
The verifier carries no basis hint byte. The square test on `1+b²` is in
`F_p`, not over the integers.

Let `P` be the index point cleared by the odd cofactor `f`, and `R` the
point with uncleared abscissa `−x(index point)−A`, also cleared by `f`.
The signature pair is `(P,Q)` with `Q=difference(P,R)` **after** clearing.
`P−Q` has abscissa `R`. In the existing Rust `EvenBasis` names these are
`even.p`, `even.difference`, and `even.q`, respectively. Lift this triple
together to fix the relative signs; a common sign does not matter.
The transitional single-chain probes are retained only in tests.

## Acceptance relation

Both public-key basis points must have exact order `2^e` and span the
public even torsion; the search alone is not enough. Both response points
must be on `E_sig`, have exact order `2^e`, and span
the full even torsion. Before constructing either chain require

`e_2^e([s]P_sig,[s]Q_sig) = e_2^e(P,Q)^q`.

Since `q ≡ −s² mod 2^e`, use the low `e` bits of `−s²` for this comparison.
Pairing constrains the determinant; it does not establish the isogeny or
bind the message by itself.

Use the symplectic product basis
`(P, [s]P_sig, Q, [q^−1 s]Q_sig)`. Build the two closed-form symplectic
matrices recorded in the archived dual-structure derivation, with
`t=s^−1 mod 2^e`. Their kernel halves give full-point lifts, not x-only
kernels. The long chain consumes order `2^e` points; the short chain
consumes their appropriate order `2^(e−1)` multiples.

Canonicalize each factor's order-four basis while preserving its pairing.
A bounded enumeration over four coefficients replaces the Sage reference's
order-four discrete logarithms. Build the product theta null and apply
`base_change_theta(M0*Mi mod 4, e4)` to each chain. The adapted gluing,
8-torsion steps and dual evaluation are ported from the pinned Theta_dim4
reference. The balanced kernel stack uses O(log e) saved generator pairs.

Require `C1.zero() == H(C2.zero())` projectively. Then evaluate
`F = dual(chain2) ∘ chain1` on `(X,0)` for `X=P,Q,P−Q` and require

`x(F(X,0)[0]) = x([s]X)` and
`x(F(X,0)[1]) = x([s]Y)`,

where `Y=P_sig,Q_sig,P_sig−Q_sig`, respectively. This is the executable
Sage model's complete two-component image criterion. The difference point
constrains relative signs. The mathematical soundness and uniqueness
argument for this signed representation remains subject to external review.

Every projective comparison rejects an all-zero coordinate vector. The
inverse gluing map also requires a nonzero product point satisfying its
rank-one relation; it does not manufacture infinities from invalid output.

## Refusals and cost

Wrong lengths precede decoding. Key checks and canonical decoding precede
Sign, response Singular/Model, bounded Basis and Composite. Then run
on-curve/order/span checks, Pairing, chain construction, Codomain and Image.
`Kernel` can occur before or after pairing, and `Singular` can refer to
either curve; enum ordering is not a global precedence promise.
`Ok(())` is returned only after every image comparison passes.

Verification has no RNG or secret inputs. Public arithmetic is variable
time. The signer remains a separate crate and process backend.

## Executable evidence

- All 113 committed honest signatures are executed through `verify`.
- Both chains match every Sage theta-null point for the six primary vectors.
- Negative tests cover signs, swaps, determinant-one shears, odd scalar
  changes, every nonzero order-four translate in either or both points,
  alternate prime challenges, other messages, all five alternative models,
  reduced orders, off-curve points, singular and exceptional models.
- Salt negatives. Every one-bit change of the salt is refused: `Composite`,
  or `Image` when the new degree is prime. Over all 113 signatures the
  split is 62 prime to 5,570 composite at I, 137 to 18,679 at III and 118
  to 21,386 at V. A salt found by search whose degree is prime gives
  `Image`. The salt of another valid signature under the same key gives
  `Composite` or `Image` (native signer test). An archived unsalted
  signature gives `Length` (`prism-compare`).
- Test builds use optimization level 1 with debug assertions and overflow
  checks enabled, so the full crypto corpus remains practical in CI.
- The native signer test generates fresh responses at all levels, checks
  byte-for-byte determinism, binary messages and message binding, with
  zero and nonzero caller randomness.
- The default build has no reduced-width entry point: CI compiles a
  consumer that must fail on `prism_verify::research` and build only with
  `research-reduced-width`. At the production width the comparison driver
  calls the production signing session, so the bytes it releases are the
  production signer's bytes.
- `scripts/sage/differential.sage` compares a fresh Sage execution against
  Rust, including the other-key cases. It requires Sage and pinned source.
- Fuzz corpora are seeded with all honest signatures, so deep paths are
  reachable. Fuzzing is not a mathematical soundness proof.

The benchmark uses an accepted honest signature at each level. Development
machine numbers are not a Raspberry Pi measurement. See
[benchmark instructions](BENCHMARKING.md).
