# Q5: the largest intermediate of the fixed-schedule Z[i]-Lagrange reduction

Date: 2026-11-24. Branch `experiment/q5-width-proof`, folder
`research/q5-width-proof/`. PRISM is the signature scheme of ePrint 2026/443
(Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux, Maino,
Vercauteren, Wesolowski); shekyl-prism implements it. The Shekyl parameter
sets, and the only ones, are P248, P324, P376, P500 and P664. Research only:
nothing here changes `dev`, the signer, `SIGNER_SPEC.md` or a parameter set.

## Summary

- **Q5 is closed for the reduction proposed, given Q4.** For the
  fixed-schedule Z[i]-Lagrange reduction (T_prov steps with the Rust
  prototype's 64-bit quotient, then one exact cleanup step, then the
  conversion back to a 4-D basis and its Gram matrix), every intermediate
  is bounded by a function of one number, M0, the larger starting value of
  the form N4 = 4·nrd on the two input vectors. The largest intermediate is
  the cleanup's rounding numerator 2h + nrd(u), at most 3·M*, where
  M* = ⌈M0·(1 + ε)^S⌉, ε < 2^-59.41, and S is the schedule length plus one
  (Theorem 1, Lemma 4). In bits: **`2·(B + 2e − 5) + 5`** on signing's
  intersection. The schedule length enters only through (1 + ε)^S, so the
  bound holds for any S up to 2^50 and does not depend on whether the
  schedule converges.
- **Per set, proven largest intermediate against the prism-int width**
  (signing, generic shape, O0 numerators): 1,303 / 1,681 / 1,947 / 2,565 /
  3,385 bits against 1,664 / 2,112 / 2,432 / 3,136 / 4,160. That leaves
  **361 / 431 / 485 / 571 / 775 bits spare**. P500 has 571. Every other
  variant fits too: key generation, the challenge ideal alone, keygen
  pre-reduced signing guarded or unguarded, and even an input of any HNF
  shape (spare 111 / 105 / 103 / 66 / 107).
- **Observed maxima agree with the proof to within 2 bits** on 22,412
  runs a set (scale 100, section 5). Families: sampled
  PRISM ideals in both coordinate systems, secrets at the bound B with the
  largest q, Rolletschek chains, corner, ping-pong and tail-stress inputs,
  bases that clamp every quotient, key generation, and pre-reduced signing
  with sampled keys and keys crafted to pass, sit at or fail the keygen
  guard. No run exceeded the bound computed from its own M0. The largest
  values are the starting norms themselves: the reduction never grows a
  norm by more than 2^-59 relative, and the observed maximum of
  log₂(M_max/M0) is 0.
- **Where the budget is exceeded: only without Q4.** If the secret norm is
  bounded only by the proven `2L + 18` bits, signing's intersection needs
  2,027 / 2,631 / 3,067 / 4,051 / 5,363 bits, which is **363 to 1,203 bits
  over** the width. Q4 (refuse N ≥ 2^B, B = ⌈L/2⌉ + 32, at key generation and
  at decode) is what makes Q5 hold. Q4 has slack: B could be as large as
  338 / 410 / 466 / 570 / 754 before the generic path overflows. For an input
  of any shape the limit is 213 / 247 / 275 / 318 / 420.
- **The prism-int LIMBS do not change.** Row 5 of the budget becomes
  `2b + 5` bits for the generic shape, where b = bits of the ideal's norm.
  That is L − 1 bits below row 3 (`2b + L + 4`), which stays the binding
  row. For an input of any HNF shape, row 5 is `2b + L + 5`. That is one
  bit over row 3 at P324, P376, P500 and P664, but ⌈need/64⌉ + 1 gives the
  same LIMBS at all five sets.
- **Two findings outside the width question** (section 6):
  1. **The 64-bit quotient's clamp breaks reduction on a valid ideal.**
     `crafted_saturation.py` builds a prime-norm ideal at signing size that
     contains an element of norm k·T, with k around 2^(L/2+20). On it, every
     64-bit step clamps, and T_prov steps plus the cleanup end with a basis
     that is not reduced. Exact quotients finish in 8 to 10 steps. Widths
     stay inside the bound (Lemma 3 covers clamps). So the T_prov
     certificate does not cover the bounded-precision schedule. For honest
     inputs it needs a partial quotient above 2^60, which is roughly
     2^-110-rare, but nothing reports it today.
  2. **The keygen guard `nrd(v_sec) ≤ 2√p·N` rejects about 22% of sampled
     keys** (2,141 to 2,272 of 10,000 at each set), not 0 of 2,500 as the
     keygen-precompute report states. It is not needed for width. A factor
     of 256 instead of 2 rejected none of 10,000.

## 1. What is proven, and for which algorithm

The algorithm is `reduce` of `research/rust-lagrange-ct/src/zi.rs`, itself
fsr.py's `zi_gauss_fixed` with `prec = 64`, run from any Z[i]-basis
`{U, V}`:

1. An initial conditional swap, so that N4(U) ≤ N4(V).
2. S − 1 fixed steps, where S − 1 is T_prov of the input class
   (301 / 387 / 446 / 587 / 773 for the secret product, 211 / 276 / 319 /
   425 / 565 for I_q, 283 / 369 / 428 / 569 / 755 for pre-reduced and
   guarded, 354 / 461 / 536 / 712 / 945 for pre-reduced and unguarded). Each
   step does the following:
   - Compute (h0, h1) = herm(V, U).
   - Compute m_j = Q64(h_j, N4(U)): the Gaussian rounding of h_j/N4(U),
     from the top 64 bits of N4(U) with h shifted by the same amount. h is
     clamped at ±2^124, the quotient is clamped at 2^62, and the
     "fits" test applies.
   - Set V ← V − m0·U − m1·iU and compute N4(V).
   - Swap if N4(V) < N4(U).
3. One exact step, m_j = ⌊(2h_j + N4(U)) / (2·N4(U))⌋ at full width.
4. The conversion back: the 4-D basis [U, iU, V, iV] and its Gram matrix,
   with entries N4(U), N4(V), ±h0, ±h1.

`instrumented.py` is this algorithm with `quotient()` ported line by line,
and it records every value the Rust code materialises, in the Rust code's
order.

**Coordinates.** The signer's maximal order is O0 = ⟨1, i, (i+j)/2,
(1+k)/2⟩ (SIGNER_SPEC 1.4.5). An element is held by integer numerators X
over 2 (Q1), so N4(X) = X0² + X1² + p(X2² + X3²) = 4·nrd. The research
models use the index-4 order Z⟨1, i, j, k⟩ with N4 = nrd; both are checked.
The headline numbers are O0's, which are the larger of the two by up to
2 bits. Left multiplication by i is (X0, X1, X2, X3) ↦ (−X1, X0, −X3, X2)
in both. Let ⟨X, Y⟩ = X0Y0 + X1Y1 + p(X2Y2 + X3Y3); then
herm(V, U) = (⟨V, U⟩, ⟨V, iU⟩).

**What is not covered.** Lehmer batching, a variable-time early exit, a
different quotient, and the construction of the starting basis. The
starting basis is either the generic HNF shape (lemma 5) or the
pre-reduced intersection. Its own arithmetic belongs to 2b and 2d:
SIGNER_SPEC rows 1 and 2, and section 4.4 here for the pre-reduced
setup.

## 2. Notation

For one step, let A = N4(U), C = N4(V), M = max(A, C), h = (h0, h1),
q_j = h_j/A and q = (q0, q1). Let M_t be M after t steps, M0 = M_0, and
δ = 2^-63. Let ε = 12δ/(1 − δ)², which is below 2^-59.41. S is the
number of steps including the cleanup, and M* = ⌈M0(1 + ε)^S⌉ (exact
rational arithmetic in `bounds.py`).

## 3. The proof

**Lemma 1 (identities).** For integers m0, m1:

- (a) N4(V − m0U − m1·iU) = C − 2(m0h0 + m1h1) + (m0² + m1²)A.
- (b) N4(A·V − h0U − h1·iU) = A·(AC − h0² − h1²). Since the left side is
  at least 0 and A > 0, **h0² + h1² ≤ AC**: Bessel's inequality for the
  orthogonal pair {U, iU} of equal norm.
- (c) Δ = AC − h0² − h1² is unchanged by the update and by the swap.

Checked symbolically (`lemmas_check.py` L1, L1b, L2, L2b, L2').

**Lemma 2 (coordinates and partial sums).**

- |X0|, |X1| ≤ √N4(X), and |X2|, |X3| ≤ √(N4(X)/p).
- Every partial sum formed in herm and N4 is at most √(N4(U)·N4(V)) in
  absolute value: X0Y0 + X1Y1, X1Y0 − X0Y1, and p times the j-part sums.
  The j-part sums before the multiplication by p are at most that over p.
  The proof is the two-term Lagrange identity
  (ac + bd)² + (ad − bc)² = (a² + b²)(c² + d²) (L2c), applied to each pair
  of coordinates.

**Lemma 3 (one step).** After a 64-bit step, N4(V') ≤ C + εM. After the
exact step, N4(V') ≤ C.

*Proof.* By Lemma 1(a), N4(V') − C = A·Σ_j m_j(m_j − 2q_j). Q64 sets
k = max(0, bitlen(A) − 64), hs = ⌊h_j/2^k⌋, ns = ⌊A/2^k⌋, and q̃_j = hs/ns.

- If k = 0, then q̃_j = q_j.
- If k > 0, then ns ≥ 2^63. Writing h_j/2^k = hs + a and A/2^k = ns + b
  with a, b ∈ [0, 1):

      |q_j − q̃_j| = |a·ns − b·hs| / (ns(ns + b)) ≤ (1 + |q̃_j|)·δ.

The output m_j comes from one of three cases:

- (i) the half-up rounding of q̃_j;
- (ii) the half-up rounding of the clamped value ±2^124/ns, which has the
  sign of q̃_j and is no larger in absolute value;
- (iii) sign(q̃_j)·2^62, when that rounding is at least 2^62 or the
  quotient is at least 2^64, so that |q̃_j| > 2^61.

In every case, **m_j(m_j − 2q̃_j) ≤ 0 and |m_j| ≤ |q̃_j| + 1/2**. For (i)
this is lemma L3a, checked exhaustively on 16,040 rationals. For (ii) it
follows because m_j·q̃_j ≥ m_j·(clamped value). For (iii) it follows
because 0 ≤ m_j/q̃_j ≤ 2. Therefore:

    m_j(m_j − 2q_j) ≤ 2|m_j|·|q̃_j − q_j| ≤ 2δ(1 + |q̃_j|)² ≤ 2δ(1 + |q_j|)²/(1 − δ)² ≤ 4δ(1 + q_j²)/(1 − δ)²

Summing over j and multiplying by A, then using A·|q|² = |h|²/A ≤ C
(Lemma 1(b)):

    N4(V') − C ≤ 4δ/(1 − δ)²·(2A + C) ≤ 12δ/(1 − δ)²·M = εM.

For the exact step, δ = 0. □

The check: the exact port of `quotient()` is run at precisions 6, 8, 10
and 64, with the clamps scaled for the toy precisions. Inputs are random,
saturating (tiny U, huge V), nearly parallel, and targeted half-integer
ties, 180,000 in all. The worst observed (N4(V') − C)/M is 4.9e-2 against
a bound of 0.40 at precision 6, and 1.2e-19 against 1.3e-18 at
precision 64 (L3). So bounded precision can grow a norm, and the bound
holds with room.

**Theorem 1 (no growth).** M_t ≤ M0(1 + ε)^t for every t, whatever the
schedule length and whether or not the schedule converges.

*Proof.* A step replaces {U, V} by {U, V'}, possibly swapped, so
M_{t+1} = max(A, N4(V')) ≤ max(A, C + εM_t) ≤ (1 + ε)M_t. The cleanup
step has ε = 0. □

So every norm in the run is at most M*. For S ≤ 2^50,
M* < M0(1 + 2^-9), so the schedule length does not affect the width.

**Lemma 4 (each intermediate, from M*).**

| Intermediate (names as in `instrumented.py`) | Bound |
|---|---|
| N4 of every vector, h0 and h1, every partial sum of herm and N4, 4-D Gram entries | M* |
| j-part sums before the multiplication by p | M*/p |
| Coordinates of U, V, iU, iV at every point, and of the output | √M* (√(M*/p) for j, k) |
| m_j·U_r in a 64-bit step | (3/2 + 4δ)·√M*, because \|m_j\| ≤ (1 + 2δ)\|q_j\| + 1/2 + 2δ, \|q_j\| ≤ √(C/A) and \|U_r\| ≤ √A |
| a_r = (μU)_r, the update term | (1 + 2δ + 1/√2 + 2δ)·√M*, from \|μ\|·√A with \|μ\| ≤ \|q̃\| + 1/√2 |
| 64-bit quotient internals | \|hs\| ≤ 2^124, ns < 2^64, \|2hs + ns\| < 2^126, \|m\| ≤ 2^62 (two limbs, fixed) |
| Cleanup quotient \|m_j\| | √(M*/A) + 1/2 ≤ √M* + 1 |
| Cleanup rounding numerator \|2h_j + A\| and denominator 2A | 3M* and 2M* |
| Gram in the reference's scaling (2× N4) | 2M* |

The largest is 3M*: bits(M0) + 2, or + 3 when (1 + ε)^S carries M0 over a
power of two.

**Lemma 5 (starting values M0).**

**(a) Generic shape.** Write O0 = Z[i] ⊕ Z[i]ω with ω = (i + j)/2. A left
O0-ideal I of odd norm T contains T·Z[i]. The projection onto the
ω-coefficient is Z[i]-linear, with kernel I ∩ Z[i]. Since
[O0 : I] = T² = [Z[i] : I ∩ Z[i]]·[Z[i] : image] (Z-indices), the case I ∩ Z[i] = T·Z[i]
forces the image to be all of Z[i]. Then I has the Z[i]-basis with
numerators U = (2T, 0, 0, 0) and V = (x, y, 1, 0), where x is even, y is
odd and both lie in [0, 2T). (L5b checks closure under ω and (1 + k)/2 on
50 ideals.) So:

    M0 ≤ max(4T², 2(2T − 1)² + p) < 8T² + p

With x and y centred mod 2T, M0 ≤ max(4T², 2T² + p). In the model order,
the bounds are T² and 2T² + p.

**When the shape is generic:**

- **The challenge ideal.** q = 2^(2e−5) − s² with s odd is 3 mod 4 (L5c),
  so q is inert in Z[i], I_q ∩ Z[i] is q·Z[i], and I_q is always generic.
- **Key generation's ideal.** D = nextprime(2^(2W)) is 3 mod 4 at all five
  sets (+75, +183, +183, +643, +75), so I_D is always generic.
- **The secret ideal.** I_sec is non-generic only if N ≡ 1 mod 4 and
  I_sec ∋ π for a prime π of Z[i] above N. Then I_sec = O0·π, a principal
  ideal whose curve is E0 itself. Key generation must refuse such a key,
  and the decoder must test the shape.
- **The intersection.** (I_sec ∩ I_q) ∩ Z[i] = N·Z[i] ∩ q·Z[i] = Nq·Z[i],
  so it is generic whenever I_sec is. Q4 gives T = Nq < 2^(B + 2e − 5).

**(b) Any shape**, every numerator in [0, 2T]: M0 ≤ (2 + 2p)(2T)².

**(c) Pre-reduced signing** (keygen-precompute EXP_RED_O0). From a stored
basis (u_s, v_s) of I_sec with N4(u_s) ≤ N4(v_s) =: C_s, the intersection
has the basis W1 = q·v_s and W2 = u_s + γ·v_s, with γ ∈ Z[i] centred mod q,
so |γ|² ≤ (q − 1)²/2. Then:

    N4(W1) = q²·C_s
    N4(W2) ≤ C_s·(1 + |γ|)²

Three bounds on C_s, with Δ_s = pN²/4 in nrd units for O0:

- **Guarded:** nrd(v_s) ≤ 2√p·N.
- **Unguarded, any shape:** C_s ≤ Δ_s/A_s + A_s/2 with A_s ≥ N, so
  nrd(v_s) ≤ pN/4 + N/2. An exactly reduced basis has |h| ≤ A/√2 and
  A_s² ≤ 2Δ_s.
- **Unguarded, generic shape:** A_s ≥ min(N², p/4), so
  nrd(v_s) ≤ max(N² + p/8, N√(p/2)).

The width proof needs only that the decoder checks
**N4(u_s) ≤ N4(v_s) ≤ (the bound used)**.

## 4. The bounds, per set (machine-checked: `bounds.py`, `results/proven_summary.csv`)

Largest intermediate (3M*), in bits, for O0 numerators. Width = 64·LIMBS
(SIGNER_SPEC 2.1.2). `need` is the spec's row 3.

| Variant | P248 | P324 | P376 | P500 | P664 |
|---|---:|---:|---:|---:|---:|
| **Signing, I_sec ∩ I_q, generic shape** (`2(B+2e−5)+5`) | **1,303** | **1,681** | **1,947** | **2,565** | **3,385** |
| same, x and y centred | 1,302 | 1,680 | 1,946 | 2,564 | 3,384 |
| Challenge ideal I_q alone | 987 | 1,291 | 1,499 | 1,995 | 2,651 |
| Key generation, ideal of norm D | 1,029 | 1,541 | 1,541 | 2,053 | 3,077 |
| Keygen pre-reduction of I_sec | 321 | 395 | 453 | 575 | 739 |
| Pre-reduced signing, guard nrd(v_s) ≤ 2√p·N | 1,270 | 1,649 | 1,914 | 2,532 | 3,352 |
| Pre-reduced, unguarded, generic I_sec | 1,302 | 1,680 | 1,946 | 2,564 | 3,384 |
| Pre-reduced, unguarded, any shape | 1,392 | 1,809 | 2,102 | 2,782 | 3,683 |
| Signing, any HNF shape (`2b+L+5`) | 1,553 | 2,007 | 2,329 | 3,070 | 4,053 |
| *Signing without Q4* (N < 2^(2L+18)) | *2,027* | *2,631* | *3,067* | *4,051* | *5,363* |
| prism-int width | 1,664 | 2,112 | 2,432 | 3,136 | 4,160 |
| `need` (row 3) | 1,553 | 2,006 | 2,329 | 3,069 | 4,053 |
| **Spare, signing generic** | **361** | **431** | **485** | **571** | **775** |
| Spare, any shape | 111 | 105 | 103 | 66 | 107 |
| *Spare without Q4* | *−363* | *−519* | *−635* | *−915* | *−1,203* |

Per intermediate (signing, generic, O0), proven bits: starting norm
2b + 3; coordinates and update terms b + 2 to b + 3; herm, N4, Gram
2b + 4; the 64-bit quotient 63, with internals 126; cleanup quotient
b + 2; cleanup numerator 2b + 5. Here b = B + 2e − 5 = 649 / 838 / 971 /
1,280 / 1,690. All rows are in `results/tables.md`.

### 4.1 Q4 and Q5 together

The signing bound uses only T < 2^(B + 2e − 5), and that is Q4. Without
Q4, the reduction overflows at every set (last rows above). With Q4, the
largest B for which the reduction still fits is
**338 / 410 / 466 / 570 / 754** for the generic shape, and 213 / 247 /
275 / 318 / 420 for any shape, against B = 158 / 195 / 224 / 285 / 367.
The narrowest is P500 with any shape: 33 bits. So Q4's rule is necessary,
and its 15 to 17 bits of headroom above the largest candidate seen are
far inside what Q5 can absorb. Q4 must hold for imported keys too, as
SIGNER_SPEC already says.

### 4.2 Row 5 of the budget, and LIMBS

The Z[i] path never forms the 4×4 Gram of the HNF basis, whose j-pivot
rows give row 3 its `+L`: its α has j-coordinate 1. So row 5 is
`2b + 5` bits, L − 1 below row 3, in the generic shape. LIMBS stays as
it is, because row 3 (if any consumer still forms that Gram) and row 6
(stage 3, `2b + L + 4`) still bind.

For an input of any shape, row 5 is `2b + L + 5`, one bit above row 3 at
P324, P376, P500 and P664. ⌈need/64⌉ + 1 still gives 26 / 33 / 38 / 49 /
65 limbs. prism-int is sign and magnitude with a symmetric range, so
"magnitude below 2^(64·LIMBS)" is the whole condition, and the sticky
overflow mark is unreachable on this path.

### 4.3 The research prototype's own widths

`rust-lagrange-ct/src/sets.rs` sizes C = ⌈(bits(T) + 3)/64⌉ coordinate
limbs and 2C norm limbs. That fits the generic shape, key generation,
and the guarded and unguarded-generic pre-reduced variants, with at least
34 coordinate bits to spare. It does **not** fit an input of any shape
(at every set), nor the unguarded any-shape pre-reduced variant at P324,
P376, P500 and P664. A prism-int port with one width per set has no such
issue. A future reduction-local width has to be sized from the variant's
M0 + 3 bits.

### 4.4 The pre-reduced setup (stage 2d, for completeness)

The products u_s[2]·x_q and the like, the values before reduction mod q,
and the coefficients of γ·v_s and of W1 and W2 are at most √M0-sized. The
largest setup value is the norm of r_v or a product t·inv, below 2q², so
2·bits(q) + 1 bits. Observed: 983 / 1,287 / 1,495 / 1,991 / 2,647, which
is exactly 2·bits(q) + 1 at each set and far below the reduction's own
values. The inverse modulo q is a q-sized extended
gcd.

## 5. Empirical validation

`run_empirical.py`, scale 100, two processes. Each run is checked two
ways:

- **(a) Lemma by lemma:** every recorded intermediate against the Lemma 4
  bound computed from that run's own M0.
- **(b) The width claim:** against the proven bound of the run's variant.

Ideal inputs are also checked against the exact Z[i]-Lagrange oracle
(final nrd(u) = λ1). Twenty sampled inputs a set are checked step for step
against fsr.py `zi_gauss_fixed(prec=64)`, unchanged.

Per set: 22,412 runs, of which 21,400 are on valid ideals (sampled, at the bound, key generation, challenge ideal, pre-reduced with sampled and crafted keys) and 1,012 on adversarial bases. The ideal oracle was checked on 12,162 runs a set.

| Check (per set) | P248 | P324 | P376 | P500 | P664 |
|---|---|---|---|---|---|
| Runs where an intermediate exceeded the bound from its own M0 (Lemma 4) | 0 | 0 | 0 | 0 | 0 |
| Runs where the largest norm grew above M0 (max log₂(M_max/M0)) | 0 (0.0) | 0 (0.0) | 0 (0.0) | 0 (0.0) | 0 (0.0) |
| Ideal runs not ending at λ1 (of 12,162 checked) | 0 | 0 | 0 | 0 | 0 |
| Steps differing from fsr.py `zi_gauss_fixed(prec=64)` (20 inputs) | 0 | 0 | 0 | 0 | 0 |
| Clamped 64-bit quotients on ideal inputs (21,400 runs) | 0 | 0 | 0 | 0 | 0 |

Largest intermediate, observed / proven bits (the variant's bound), and observed spare to the width:

| Variant | runs (P500) | P248 | P324 | P376 | P500 | P664 |
|---|---:|---|---|---|---|---|
| Signing, generic, O0 numerators (sampled, at the bound, max-norm, O0-scaled Rolletschek, max entries) | 3603 | 1301 / 1303 (spare 363) | 1679 / 1681 (spare 433) | 1945 / 1947 (spare 487) | 2563 / 2565 (spare 573) | 3383 / 3385 (spare 777) |
| Signing, generic, model order (incl. all adversarial bases) | 9609 | 1299 / 1301 (spare 365) | 1677 / 1679 (spare 435) | 1943 / 1945 (spare 489) | 2561 / 2563 (spare 575) | 3381 / 3383 (spare 779) |
| Key generation, norm D, O0 | 500 | 1027 / 1029 (spare 637) | 1539 / 1541 (spare 573) | 1539 / 1541 (spare 893) | 2051 / 2053 (spare 1085) | 3075 / 3077 (spare 1085) |
| Challenge ideal alone | 1000 | 983 / 985 (spare 681) | 1287 / 1289 (spare 825) | 1495 / 1497 (spare 937) | 1991 / 1993 (spare 1145) | 2647 / 2649 (spare 1513) |
| Keygen pre-reduction of I_sec | 1000 | 317 / 319 (spare 1347) | 391 / 393 (spare 1721) | 449 / 451 (spare 1983) | 571 / 573 (spare 2565) | 735 / 737 (spare 3425) |
| Pre-reduced signing, keys passing the guard | 3240 | 1267 / 1268 (spare 397) | 1645 / 1647 (spare 467) | 1910 / 1912 (spare 522) | 2529 / 2530 (spare 607) | 3349 / 3350 (spare 811) |
| Pre-reduced signing, keys failing the guard (sampled and crafted) | 2460 | 1298 / 1300 (spare 366) | 1676 / 1678 (spare 436) | 1942 / 1944 (spare 490) | 2560 / 2562 (spare 576) | 3380 / 3382 (spare 780) |

Per intermediate, signing, generic shape, O0 (proven / observed bits):

| Intermediate | p248 | p324 | p376 | p500 | p664 |
|---|---:|---:|---:|---:|---:|
| starting norms N4 (in) | 1301 / 1301 | 1679 / 1679 | 1945 / 1945 | 2563 / 2563 | 3383 / 3383 |
| coordinates, step and conversion | 651 / 650 | 840 / 839 | 973 / 972 | 1282 / 1281 | 1692 / 1691 |
| quotient x coordinate (m u) | 652 / 651 | 841 / 840 | 974 / 973 | 1283 / 1282 | 1693 / 1692 |
| update temporaries a = mu u | 652 / 651 | 841 / 840 | 974 / 973 | 1283 / 1282 | 1693 / 1692 |
| Hermitian products / partial sums / h | 1302 / 1300 | 1680 / 1678 | 1946 / 1944 | 2564 / 2562 | 3384 / 3382 |
| j-part sums before x p | 1051 / 802 | 1354 / 1030 | 1563 / 1183 | 2059 / 1555 | 2715 / 2049 |
| norms N4 and partial sums | 1302 / 1300 | 1680 / 1678 | 1946 / 1944 | 2564 / 2562 | 3384 / 3382 |
| 64-bit quotient |m| | 63 / 61 | 63 / 61 | 63 / 61 | 63 / 61 | 63 / 61 |
| 64-bit quotient internals (hs, num) | 126 / 126 | 126 / 126 | 126 / 126 | 126 / 126 | 126 / 126 |
| cleanup exact quotient |m| | 651 / 401 | 840 / 515 | 973 / 591 | 1282 / 778 | 1692 / 1024 |
| cleanup rounding numerator 2h + nu | 1303 / 777 | 1681 / 1004 | 1947 / 1165 | 2565 / 1535 | 3385 / 2027 |
| cleanup rounding denominator 2 nu | 1303 / 777 | 1681 / 1004 | 1947 / 1165 | 2565 / 1535 | 3385 / 2027 |
| 4-D Gram entries (N4 scale) | 1302 / 1300 | 1680 / 1678 | 1946 / 1944 | 2564 / 2562 | 3384 / 3382 |
| 4-D Gram entries (reference 2x scale) | 1303 / 1301 | 1681 / 1679 | 1947 / 1945 | 2565 / 2563 | 3385 / 3383 |

Reading the tables:
- The observed maxima sit 1 to 2 bits under the proof, because the starting norms reach their bound: secrets at 2^B with the largest q, and the max-entry inputs. The reduction itself never added a bit.
- The cleanup's own values (quotient, numerator) are far below their bound. The bound allows for a cleanup that meets an unreduced basis, and the observed ones met reduced ones.
- The clamped quotients in the model row come only from the adversarial bases (corners, max entries, saturation), at 2 per step.
- Keygen guard on 1,000 sampled keys a set: 759 to 803 passed and 197 to 241 failed. All 600 crafted skew-max and edge-fail keys failed, and 1,383 of 1,500 edge-pass keys passed. Each key then signed three challenges.

Every row, by family and intermediate: `results/empirical_vs_proven.csv`; aggregates: `results/summary.json`, `results/tables.md`.

## 6. Findings outside the width question

### 6.1 The 64-bit quotient's clamp: a valid ideal it fails on

The prototype clamps h at ±2^124 and the quotient at 2^62. Its comment
says "|h/nu| > 2^60 never occurs in a reduced step on these lattices; if
it did the step would still be a valid unimodular move, only a shorter
one". The step is valid, and Lemma 3 shows it cannot grow anything. But a
clamped step makes additive progress of at most 2^62 on a quotient that
may be 2^300, so the schedule stalls.

`crafted_saturation.py` takes T prime at the full signing size and an
element β = a + bi + j of norm k·T, with k ≈ 2^(L/2+20) (Cornacchia on
kT − p). The model ideal {T, (a mod T) + (b mod T)i + j} is then a valid
left ideal of prime norm T. Results (`results/crafted_saturation.txt`):

| Set | Exact early-exit steps | 64-bit schedule: clamped quotients | Final = λ1 | Same schedule, exact quotients |
|---|---:|---:|---|---|
| P248 | 10 | 602 | **no** (791 vs 774 bits) | yes |
| P324 | 9 | 774 | **no** (1,019 vs 998) | yes |
| P376 | 8 | 892 | **no** (1,176 vs 1,161) | yes |
| P500 | 8 | 1,174 | **no** (1,547 vs 1,532) | yes |
| P664 | 9 | 1,546 | **no** (2,039 vs 2,024) | yes |

The width bound held on every one of these runs, with no violations.

The corner, max-entry and saturation adversaries of section 5 clamp too.
They end at λ1 only because their one large quotient is the last, and the
exact cleanup takes it.

On honest inputs, clamping needs a partial quotient above 2^60, which
means λ2²/λ1² above roughly 2^120 at some step. For Gaussian continued
fractions P(|q| ≥ X) falls like 1/X², which gives roughly 2^-110 over a
schedule. No ideal input clamped (0 of 21,400 ideal runs a set).
The inputs to signing are an honest key and a hash-derived I_q, which an
attacker does not choose.

So this is a negligible-probability correctness gap, not a width
problem. But it is silent, and it is secret-dependent. Options:

1. Make a clamp set the sticky overflow mark of prism-int, so it is
   reported like an overflow, and add it to O33 with its probability
   bound.
2. Widen the quotient to a multi-limb value bounded by
   √(M*/A_min) ≈ 2^((b+1)/2), which costs about as much as making every
   step exact.
3. Prove an explicit bound on the probability for honest key and
   challenge distributions.

Note that the T_prov certificate covers exact rounding only. The
rust-lagrange-ct report already says so, and this is a concrete
counterexample for the bounded-precision variant on a valid ideal.

### 6.2 The keygen guard rejects about 22% of keys

keygen-precompute adopts `nrd(v_sec) ≤ 2√p·N` and reports it "satisfied
by 2,500 / 2,500 sampled keys". On 10,000 keys a set, half with N at the
bound B and half with varied N, exactly Lagrange-reduced, the rejections
were:

| Set | c = 2 | c = 4 | c = 16 | c = 256 | c = 2^16 | max nrd(v_s)/(√p·N) |
|---|---:|---:|---:|---:|---:|---:|
| P248 | 2261 of 10000 (22.6%) | 494 | 32 | 0 | 0 | 77.73 |
| P324 | 2228 of 10000 (22.3%) | 512 | 39 | 0 | 0 | 92.41 |
| P376 | 2141 of 10000 (21.4%) | 474 | 30 | 0 | 0 | 72.40 |
| P500 | 2232 of 10000 (22.3%) | 519 | 34 | 0 | 0 | 111.87 |
| P664 | 2272 of 10000 (22.7%) | 548 | 32 | 0 | 0 | 153.71 |

The volume heuristic gives the right order: a 4-D lattice of covolume Δ
has a Z[i]-line of norm below √Δ/2 with probability at most about
π²/32 ≈ 0.31. Failing the guard means roughly nrd(u_s) < √Δ/2. That
report's figure appears to come from a looser coordinate-bit test (its
"max coord bits(v_sec) ≤ L/2 + B").

The guard is **not** needed for width with prism-int's LIMBS: the
unguarded variants fit with 354 or more bits spare. It matters only for
the 18-step T_prov saving and for the narrower widths of the research
prototype. If it is adopted, c = 256 rejected none of 10,000 keys a set. The tail
falls like 1/c², so it would reject about 10^-5 of keys. It costs a
few steps of T_prov, since each factor of 2 in c adds about 0.57 steps.
Its retry is a key-generation loop, and the rejection rate above is what
O33 should record for it.

### 6.3 Integration notes for 2c

The 64-bit quotient needs the bit length of nrd(u) and shifts by that
amount. Both are secret. prism-int's `SecretInt` has neither: its shifts
take a public count, and it has no bit length. The prototype does both
with a fixed barrel shifter of public-count stages plus mask selects. A
port needs those operations in `SecretInt`, or the quotient computed on
limbs outside it. This is a design item for 2c, not a width one.

## 7. Proposed text for SIGNER_SPEC (not applied: `dev` is not touched)

> **Q5 (2c), resolved for the fixed-schedule Z[i]-Lagrange reduction.**
> Every intermediate is at most 3·⌈M0(1 + 2^-59.41)^S⌉, where M0 is the
> larger N4 (= 4 nrd, numerators) of the starting Z[i]-basis and S is the
> number of steps. For the generic HNF shape of an ideal of norm T,
> M0 < 8T² + p, so the largest intermediate has at most 2·bits(T) + 5
> bits. Row 5 is `2b + 5` (generic shape) or `2b + L + 5` (any shape), and
> LIMBS is unchanged. Conditions: Q4 holds at key generation and on decode.
> The secret ideal is not principal (I_sec ≠ O0·π), which key generation
> and the decoder test. A pre-reduced key is decoded only if
> N4(u_s) ≤ N4(v_s) ≤ its stated bound. Proof and check:
> `research/q5-width-proof`.

O33 then reduces to two things: Q4's rejection rate, and the clamp of
section 6.1 (or its removal).

## 8. Caveats

- **The model and the signer.** The empirical runs use the model order
  and O0 numerators built from model ideals. Pre-reduced signing is run
  in the model only. The proof is representation-free (it uses only N4,
  herm and the multiplication by i), and the headline numbers are for O0.
  No ideal produced by our own key generation was used, because none
  exists yet.
- **Algorithm scope.** The proof covers the algorithm of section 1 only.
  A Lehmer-batched or otherwise reorganised reduction needs its own
  Lemma 4, though Theorem 1 carries over to any sequence of steps that
  meets Lemma 3.
- **The starting basis.** Its construction is assumed to give the
  generic shape (lemma 5) or the pre-reduced form. A non-generic secret
  ideal is excluded by a key check, not by the reduction.
- **Prime test.** "D is 3 mod 4" uses the probable prime D of 1.4.6.
  The primality certificates are another experiment.
- **No constant-time claim** is made or tested here.

## 9. Reproduction

See `README.md`. `lemmas_check.py` (about 11 s), `bounds.py` (under 1 s),
`crafted_saturation.py` (about 1 s), `run_empirical.py --scale 100`
(about 20 minutes on two cores of the shared box), and `analyze.py`.
