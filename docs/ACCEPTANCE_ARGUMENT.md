# Conditional acceptance argument for review

This note is a proposed algebraic argument for the implemented relation,
not an independently checked proof or a SUF-CMA theorem. In particular,
matching theta-null coordinates must identify the correctly polarized
codomain with the intended level structure. Establishing that the adapted
construction supplies this identification is a review obligation.

Let `r=e′=e−2`, `N=2^e=2^(r+2)`, `D=2^(2r−1)`, and
`0<s<2^(r−1)` odd. Assume `q=D−s²` is actually prime; a BPSW verdict alone
is not a proof of this premise.

## Premise about the computed maps

Assume that the two chains are separable polarized `(2^r,2^r)` and
`(2^(r−1),2^(r−1))` isogenies, and that the Hadamard codomain identification
preserves the polarizations. Their composite with the short chain's dual
is then an endomorphism `F` of `E_vk × E_sig` satisfying

`F†F=[D]`,

where `†` is the Rosati adjoint for the product polarization. Write the
first column of `F` as `(α,β)` with `α:E_vk→E_vk`, `β:E_vk→E_sig`.
The first diagonal identity gives

`deg(α)+deg(β)=D`.

The degree interpretation includes zero homomorphisms with degree zero.
This identity and the correctness of the implemented chains and duals are
necessary premises, not conclusions established by a successful test.

## Full rational torsion validates the curve class

The checked basis implies that `N²` divides the curve's rational point
count over `F_p²`. The Hasse interval is `[(p−1)²,(p+1)²]`, of width `4p`.
Here `N²>4p` and `(p+1)²=f²N²`. Thus the upper endpoint is the only
multiple of `N²` in that interval: the point count is `(p+1)²`, so the
curve is maximal supersingular. This reasoning requires a nonsingular
curve and an actual independent order-`N` basis, not merely two abscissas
returned by the deterministic search.

## The three image points force consistent signs

The verifier checks full, independent order-`N` bases `(P,Q)` and `(U,V)`
on the two curves. Its x-coordinate comparisons mean that

`α(P)=εP·sP`, `α(Q)=εQ·sQ`,
`α(P−Q)=εD·s(P−Q)`,

with each `ε` in `{−1,1}`. Linearity gives

`(εP−εD)sP + (εD−εQ)sQ = 0`.

Because `(P,Q)` is a basis, `s` is invertible modulo `N`, and `N>2`, both
coefficients vanish modulo `N` only when all three signs agree. Thus
`α=ε[s]` on the full subgroup `E_vk[N]`. The same reasoning with the
independent response basis `(U,V)` shows that `β` acts as one consistent
sign times `s(U,V)` on the two public generators. The checks do not need
the sign of the first component to equal the sign of the second component.

This uses the response difference `U−V`, not an independently selected
third response. It also uses full order and span of the public basis;
the index search alone does not establish those conditions.

## The torsion action determines the top-left homomorphism

For elliptic-curve homomorphisms, the degree norm satisfies

`deg(α−ε[s]) ≤ (sqrt(deg(α))+s)² ≤ (sqrt(D)+s)²`.

With these parameters,

`(sqrt(D)+s)² < (1/sqrt(2)+1/2)²·2^(2r) < 2^(2r+4)=N²`.

But a nonzero separable-degree homomorphism killing every point of
`E_vk[N]` has degree at least the subgroup's order `N²`; the same kernel
size lower bound holds for total degree. Since `N` is coprime to the
characteristic, this subgroup has `N²` distinct points. Therefore
`α−ε[s]` must be zero as a homomorphism, not merely on the sampled torsion.
It follows that

`deg(β)=D−s²=q`.

The second-component comparisons establish that the carried response is
`[s^−1]β(P),[s^−1]β(Q)` up to a common sign. They prevent accepting a
basis whose low-order lifts do not describe this map's full torsion action.

## Representation uniqueness is conditional on the same map

If two degree-`q` isogenies to the same model agree on this full torsion
basis, their difference kills `E_vk[N]` and has degree at most `4q<N²`.
They are therefore the same homomorphism. The same argument applies up to
a common sign. This does not rule out an adversary finding a genuinely new
prime-degree isogeny: that is the cryptographic hardness assumption.

For curves with `j` different from 0 and 1728, the automorphisms fixing the
origin are `±1`. Selecting the least Montgomery coefficient and jointly
normalizing the response sign should therefore select one byte encoding
for a fixed isogeny up to codomain isomorphism. Exceptional automorphism
classes are excluded by the current decoder policy. Verify that the
coefficient enumeration covers exactly the required rational isomorphism
class and that no sign convention in basis recovery changes this argument.

## What this argument does not establish

It does not prove the theta construction premise, the validity of treating
an adversarial BPSW degree as prime, a reduction for accepting arbitrary
carried indices, adequate concrete challenge entropy, secure key sampling,
side-channel resistance, or SUF-CMA of the whole scheme. Those items remain
in [SECURITY_REVIEW.md](SECURITY_REVIEW.md). The implementation tests provide
agreement and regression evidence for the premise, not a substitute for it.
