# Research verifier specification

Status: executable prototype, 2026-11-12. This is a local instantiation of
PRISM's prime-degree §5 construction, not an adopted standard or a reviewed
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
| Signature bytes | 322 | 482 | 642 |

I/III/V identify the paper's field rows. They do not certify the security
level of this modified, unsalted protocol. `CHAIN_STEPS` is `e′`; `E` is
always the rational torsion exponent. `CHALLENGE_BITS` is the actual
`e−3` width, including the forced odd bit. The historical `e−1` constant
had the same byte width but overstated the number of challenge bits.

A public key is the canonical little-endian fp2 encoding of the Montgomery
coefficient `A`, for `y²=x³+Ax²+x`. The first component is real, the second
imaginary, each an integer in `[0,p)`. Singular keys and `A=0` are refused.
The accepted key bytes are its identity for the challenge hash.

The signature is `A_sig || Px || Py || Qx || Qy || LE16(j)`, five fp2
encodings and one unsigned 16-bit index. No component is reduced during
decoding. Truncation, extension and out-of-range field elements are refused.
The index has no alternate encoding. Salt-PRISM encodings are not accepted.

## Challenge

`s_j = cSHAKE256(S="shekyl-prism-challenge-v1";
A_bytes || message || LE16(j))`, truncated to `e−3` bits with bit zero set.
There is no ambiguity between the message and the fixed-width key/index.
`q_j = 2^(2e−5) − s_j²`.

The degree test is deterministic Baillie–PSW: small-prime sieve, perfect
square rejection, strong base-2 Miller–Rabin and strong Lucas–Selfridge.
This is a probable-prime predicate, not a primality certificate. Its
adversarial adequacy is an explicit unresolved assumption.

The research signer examines indices in ascending order, testing only
those for which the degree predicate passes. It returns the first exact
signature accepted by Rust, with a bound of 16 prime candidates and the
16-bit index range. A backend failure is a typed signing error, not a draw
of fresh randomness. The verifier tests only the carried index. It does
not rerun or certify the signer's search. The reduction for this interface
is still an open review item.

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
- Test builds use optimization level 1 with debug assertions and overflow
  checks enabled, so the full crypto corpus remains practical in CI.
- The native signer test generates fresh responses at all levels, checks
  byte-for-byte determinism, binary messages and message binding.
- `scripts/sage/differential.sage` compares a fresh Sage execution against
  Rust, including the other-key cases. It requires Sage and pinned source.
- Fuzz corpora are seeded with all honest signatures, so deep paths are
  reachable. Fuzzing is not a mathematical soundness proof.

The benchmark uses an accepted honest signature at each level. Development
machine numbers are not a Raspberry Pi measurement. See
[benchmark instructions](BENCHMARKING.md).
