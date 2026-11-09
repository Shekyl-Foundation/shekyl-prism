# PRISM verifier spec (draft)

Status: draft, written to be taken to the authors. This file is not a
standard. Numbers below are copied from the working copy
`shekyl-dev/docs/papers/2026-PRISM_signature.pdf` (ePrint 2026/443, Table 2
and §5 Algorithms 2–3). The SQIsign parameter source is
`shekyl-dev/docs/papers/SQIsign-20250205.pdf`. The re-randomization questions
are read from `shekyl-dev/docs/papers/2026-2312_ECLIPSE_PRISM.pdf`
(ePrint 2026/2312).

The candidate authorization level, if a later design round adopts one, is
NIST III. NIST I and NIST V are implemented so the same binary can time them.

## Parameter rows

Table 2. `p = f·2^e − 1`. Each `p ≡ 3 (mod 4)`.

| | NIST I | NIST III | NIST V |
| --- | --- | --- | --- |
| Prime | `5·2^248 − 1` | `65·2^376 − 1` | `27·2^500 − 1` |
| `e` | 248 | 376 | 500 |
| `λ` | 128 | 192 | 256 |
| Public key | 64 bytes | 96 bytes | 128 bytes |
| §5 chain length | 248 | 376 | 500 |
| salt-PRISM `a` | 192 | 256 | 320 |
| salt-PRISM signature | 191 bytes | 272 bytes | 350 bytes |
| Table 5 verify, Intel Core Ultra 7 | 9.4 Mcycles | 23.0 Mcycles | 42.2 Mcycles |

A level is a type. A level I encoding is refused by the level V decoder.

The public key is the Montgomery coefficient `A` of `E : y^2 = x^3 + A x^2 + x`
over `F_{p^2}`, little-endian canonical `fp2` encoding (`(bit length + 7) / 8`
bytes for each of the two `F_p` components). It is the key's identity, and
it is what the challenge hashes. `A = 0` is the starting curve `E0`, whose
endomorphism ring is public, so anyone could sign for it. That key is
refused (`Origin`). `A^2 ≠ 4`, so the curve is nonsingular. A non-canonical
field element is a refusal.

The verifier derives the basis index itself, in the same order as
`find_nqr_factor` and `find_nA_x_coord` in the C tree. Square tests use the
norm to `F_p` and a Legendre symbol.

- When `A` is a square: `b` from 1 upward. Skip `b` when `1 + b^2` is a
  square in `F_p`. Accept the first `b` for which `x(P) = −A / (1 + i·b)` is
  on the curve. The test is the C's inversion-free form, `A^2(z − 1) − z^2`
  a non-square for `z = 1 + i·b`: with `z` of non-square norm and `A` a
  square, the cofactor `A/z^3` of the curve equation is a non-square, so
  that character is the opposite of the curve equation's. The same identity
  makes the C's debug-build check `x(P)` a non-square a consequence rather
  than a test, so this verifier does not spend a Legendre symbol on it.
- When `A` is not a square: `n` from 1 upward. Accept the first `n` for
  which `x(P) = n·A` is on the curve.

The byte the C function returns is `(index << 1)` with the low bit set when
`A` is a square. It is not stored in the key, so the seven-bit width of that
byte is not a ceiling on the search: indices above 127 are searched too,
which is where the C's own fallback lives. The order-`2^e` test of the
resulting points is part of Algorithm 3 and is not yet run.

### The search covers the reference's fallback, under a cost bound

Ruled 2026-11-08, revised the same day. The search runs from index 1 to `2λ`
— 256, 384, 512 — and absence within that bound is a refusal (`Basis`). It
reaches the same index the C reference does, by one search rather than two.

`find_nqr_factor` and `find_nA_x_coord` loop without a bound and compress
the index into seven bits, returning `hint = 0` when it does not fit.
`ec_curve_to_basis_2f_from_hint` reads a stored index directly and reads the
zero as "search again from `start = 128`". Both of its paths name the
*smallest* qualifying index: a stored index is smallest because generation
searched from 1, and the restart is smallest at or above 128 precisely
because every index below 128 has already failed. A single search from 1
therefore answers both cases, and the verifier needs no hint byte and no
second search to agree with the reference.

The one divergence left is the bound itself, which is a cost bound and not
an encoding one. A verifier cannot loop without a bound, because whoever
picks `A` picks the search length. `2λ` is chosen so that the divergence is
no likelier than breaking the level:

| Branch | Trials within `2λ` candidates | Probability no index qualifies |
| --- | --- | --- |
| `A` not a square | all `2λ`, each on the curve with probability about 1/2 | about `2^-2λ` |
| `A` a square | about `λ`, the rest spent on the `1 + b^2` non-square filter | about `2^-λ` |

So a key the C tree generates and this verifier refuses arises with
probability at most about `2^-λ`, which is the level's own security target
and the bar the C's comment aims at when it calls the fallback negligible.
The superseded ceiling of 127 was weaker than both: it put the square branch
at about `2^-63` at every level, because only about 63 of the 127 candidates
survive the filter.

The worst-case cost is `2λ` Legendre symbols, a length the sender chooses
with `A`. Measured over 4000 random `A` per level on the development
machine, alongside the `prism-bench` median for the whole of `verify`:

| Level | Mean index | Per candidate | Worst case at `2λ` | `verify` median | Worst case as a share |
| --- | --- | --- | --- | --- | --- |
| NIST-I | 4.8 | 1.64 µs | 0.42 ms | 1.56 ms | +27% |
| NIST-III | 5.3 | 2.62 µs | 1.01 ms | 2.28 ms | +44% |
| NIST-V | 3.8 | 4.56 µs | 2.34 ms | 3.08 ms | +76% |

An honest key costs the mean, 8–17 µs. The worst case is reachable on
purpose — a sender picks `A` — and raising the bound from 127 to `2λ` raises
it from about 0.58 ms to 2.34 ms at NIST V. That is a cost increase and not
a category change: the ceiling stays below the cost of one `verify`, so a
crafted `A` can at most roughly double the work a junk signature causes, and
a node must rate-limit at `ms` scale either way. The ordering below keeps
even that off the path of most junk: every free refusal runs first.

The reopening criterion is a deployment measurement where that doubling
matters, at which point the bound moves and the probability above moves with
it.

## Variant

The variant this verifier will accept, once Algorithm 3's image check is
complete, is §5, not salt-PRISM (§6). salt-PRISM is EUF-CMA. §5 aims at
strong unforgeability because a malleable signature would make a malleable
transaction id. The C reference implements salt-PRISM only.

Algorithm 3 (`VerIsogenyStrong`) takes `E_vk`, `(E_sig, P_sig, Q_sig)`, and
an odd integer `s` with `0 < s < 2^{e−1}`:

1. Refuse unless `y(P_sig) <_{F_{p^2}} y(−P_sig)`. The order compares the
   real part as an integer in `[0, p)`, then the imaginary part. `y = 0`
   is an order-2 point and is refused: it cannot be a basis image. This
   follows Algorithm 2's normalization. Algorithm 3 line 1 states the
   opposite inequality; see the open questions.
2. Let `q = 2^{2e−1} − s^2`. Refuse unless `q` is prime. The verifier tests
   exactly one `q`, the one named by the carried index.
3. The isogeny check is Algorithm 3 steps 3–8: a `(2^e, 2^e)` chain on
   `E_vk × E_sig` in the theta model (gluing from the product, `(2, 2)`
   steps, splitting, point evaluation), a basis of `E_vk[p+1]` pushed
   through `Φ`, and acceptance only when `[2^{e−1}] Ψ(R') = [s] R'` and
   the same holds for `T'`. The chain length is `e`, not salt-PRISM's `a`.

Step 3 is the accept condition. `chain` matches
`theta_chain_compute_and_eval` on two length-160 kernels, each the graph of
an endomorphism of `E0`: one with the extra 8-torsion the reference's
verify path passes, and one without it, which is the path Algorithm 3 uses
because a rational point has order dividing `2^e` and not `2^{e+2}`. Both
were run again with the isotropy checks on, and the codomain and the image
still match. The vectors are `prism-isogeny/testdata/chain_lvl1.txt` and
`chain_lvl1_order.txt`. The salt-PRISM wrapper around that function is not
what this verifier conforms to.

`verify` then runs one chain of length `e` on
`K = ⟨(P_vk, −P_sig), (Q_vk, −Q_sig)⟩`, and does not split the codomain.
The chain is evaluated at `([s]X, 0)` and `([2^{e−1}]X, 0)` for
`X ∈ {R, T, R−T}`. `(R, T)` is a basis of `E_vk[p+1]`, specified under
"How step 8 compares points". The images of `([s]X, 0)` are read at step
`e − 1`. The images of `([2^{e−1}]X, 0)` are read at step `e`. Those two
steps are the same surface `A` in two theta structures. A signature point
that is not on `E_sig`, or a chain that stops before both readings, is
`Kernel`. Both readings existing is `Unidentified`: the transformation
between the structures is open, so the comparison is not decided and the
function does not return success. The degree-`q` witness in
`prism-verify/testdata/e0_endomorphisms.txt` does not reach that refusal:
at step 3 a theta coordinate of the step's kernel vanishes, and the
(2,2) step refuses. The reference calls that vanishing an unexpected
split. Both readings are not produced, so the result is `Kernel`.

`A` is not required to be a product of elliptic curves. salt-PRISM's chain
splits because it embeds a full isogeny whose codomain is a product. §5's
half-isogenies end at the midpoint of an odd-exponent endomorphism, which
is in general the Jacobian of a genus-2 curve. The kernel
`fixed_degree_isogeny` builds does split; that is a chain known-answer
test, `prism-isogeny/testdata/e0_fixed_lvl1.txt`, not Algorithm 3. The
degree-`q` witness is `prism-verify/testdata/e0_endomorphisms.txt`.

Table 5's 23.0 Mcycles is salt-PRISM. There is no published timing for §5.
The floor-device gate is a level III run of this chain. The bench sample
stops at `Kernel` and is not that run.

## Challenge index

Ruled 2026-11-07. The challenge degree is found by a counter the signer
searches and the signature carries. The verifier tests exactly one `q`.

- `s_j = cSHAKE256(N = "", S = "shekyl-prism-challenge-v1"; public_key_identity ‖ message ‖ LE16(j))`,
  read as `⌈(e−1)/8⌉` bytes little-endian, low bit forced to 1, bits at and
  above `e−1` cleared. The top byte keeps `((e−2) mod 8) + 1` bits, which is
  `1..=8`: a width that is a multiple of eight keeps the whole byte, and no
  width keeps none of it, so the projection needs no special case. The
  customization string stays `-v1`: nothing has consumed it.
- The public key's identity is the byte string the key decoder accepted, at
  `PUBLIC_KEY_LEN` bytes. The signer and the verifier reach the hash through
  the same decoder, so neither can absorb a different string.
- `q_j = 2^{2e−1} − s_j^2`.
- The signer takes the smallest `j` in `0..=65535` for which `q_j` passes
  Baillie–PSW, so signing is deterministic for a given key and message.
  Exhausting the range is a signing error, not a fallback.
- The signature carries `j` as 2 bytes little-endian. It does not carry `s`.
  The verifier derives `s_j`.
- The verifier does not check that `j` is the smallest. It derives `s_j`,
  tests `q_j` once, and refuses on composite.

The hit rate per candidate is low, so recomputing the search in the verifier
lets an attacker grind messages whose search runs long and make every node
pay for it. Carrying `j` makes the verifier's prime test one degree. A
different `j` is a different degree, so a third party still cannot produce a
second signature for a signed message without forging.

## Primality

Primality is Baillie–PSW: trial division by the primes up to 100, a perfect
square test, one strong Miller–Rabin test at base 2, then the strong Lucas
test with Selfridge's parameters (`D` from `5, −7, 9, −11, …` until the
Jacobi symbol is −1, `P = 1`, `Q = (1−D)/4`). Write `n + 1 = d·2^r` with
`d` odd. The strong Lucas test accepts when `U_d ≡ 0` or `V_{d·2^t} ≡ 0`
for some `0 ≤ t < r`. No random bases. The arithmetic is variable-time
Montgomery multiplication on 64-bit limbs: the degree is public.
`ibz_probab_prime(n, 20)` in the C tree is not this test. Accepting a
composite `q` would let the isogeny factor into small degrees.

Two is in the trial-division table, so parity is the table's own first step
and not a case beside it. Each table entry is compared against the whole
integer, not its low limb: a wide `n` whose least significant 64 bits spell
a small prime is not that prime, and the division is what refuses it. An
integer wider than the module's 1024 bits is refused rather than truncated,
and each level's `2e−1` is asserted to fit that width at compile time.

The C reference stores a salt in a `uint32_t`, absorbs `sizeof(uint32_t)`
native-endian bytes in `hash_to_challenge`, and when the value is
`0xFFFFFFFF` draws a fresh salt inside the verifier. That is a records-was
quirk of `external/PRISM_v2` at
`87fad7da5f12f79112521677f4f5530672afe9ba`. Our verifier has no randomness
argument. §5's `s` is not salt-PRISM's salt. When a salt is used (§6), the
paper sets `k = 2λ` bits. This decoder does not accept a `uint32_t` salt.

## Encoding of the research signature

This layout is the research encoding after the challenge-index ruling. It is
not frozen. It is not Table 2's salt-PRISM size, not the paper's PRISM-sig
sizes 189 / 288 / 388, and not the C reference's 260/388/516-byte encoding.
A signature of those C lengths is a refusal. Point compression is a later
step; bytes per input are half of the authorization-leg cost question.

| Field | Contents |
| --- | --- |
| `A_sig` | Montgomery coefficient of `E_sig`, one `F_{p^2}` element |
| `P_sig` | affine `x`, `y`, each one `F_{p^2}` element |
| `Q_sig` | affine `x`, `y`, each one `F_{p^2}` element |
| `j` | the challenge index, 2 bytes little-endian |

NIST I is 322 bytes, NIST III is 482 bytes, NIST V is 642 bytes. Wrong
length or a non-canonical element is a refusal.

## Open questions

Not built. ePrint 2026/2312 instantiates `RandPK` / `RandSK` on its own
construction, the prime-degree variant plus the §6 salt, and §9 cites
statistical unlinkability against an observer who does not know the
randomiser, for two hops. Its abstract leaves a formal SUF-CMA argument
to future work.

- Unlinkability of a re-randomized per-output key is that paper's claim.
  This workspace does not implement re-randomization and does not treat
  the claim as established for a Shekyl output key.
- Whether re-randomization holds for the unsalted §5 variant this
  verifier is aimed at is still open. ECLIPSE's randomiser is not that
  variant.

For the authors of ePrint 2026/443:

1. §5.1 estimates that a single challenge degree is prime 6.0% / 4.3% /
   3.4% of the time at NIST I / III / V. Over 4,000 odd candidates per
   level the measured rates are 0.95% / 0.58% / 0.43%. The elementary
   estimate is the prime density near `2^{2e−1}`, times 2 because `q` is
   odd, times 3/2 because 3 never divides `q`. The paper's figure is about
   seven times that. Expected signer search length is about 105 / 174 /
   235 candidates.
2. Algorithm 2 line 8 and Algorithm 3 line 1 state opposite inequalities
   for `y(P_sig)` and `y(−P_sig)`. This verifier follows Algorithm 2.
3. The §5 strong-unforgeability argument assumes a deterministic
   `H_squares` (Remark 8). Whether the reduction holds when the signature
   carries the search index is theirs to answer.
4. Can the theta structure at step `e − 1` and the theta structure at
   step `e` on `A` be related, by matching conditions or by an explicit
   matrix in `Sp_4(Z/4Z)`, using only the rational `2^e`-torsion? The
   reading under Identification concludes that the published method cannot: the
   inheritance along one chain needs `2^{e+2}`-torsion (2024/1180 §4.1),
   the explicit matrix needs a symplectic basis of `A[4]` and the image
   of `(E_vk × E_sig)[2^e]` at step `e − 1` is `(Z/2^e)^2 × (Z/2)^2`, and
   the half-torsion cutting (§5.3, Lemma 29) identifies two chains from
   the product rather than these two steps. A per-factor sign is a
   separate question. SQIsignHD's image check accepts one, on Lemma
   5.7.1 (Appendix A.4), whose proof was not available for this reading.

## How step 8 compares points

These two rulings are how step 8 reads the chain output. The chain
function now matches the reference on the vectors named above; the
comparison below is what `verify` requires of that output.

### Identification

`Φ = Φ̂₂ ∘ Φ₁` is an endomorphism of `E_vk × E_sig` of degree `2^{2e−1}`.
`Φ₁` is a `(2^e, 2^e)`-isogeny and `Φ₂` is a `(2^{e−1}, 2^{e−1})`-isogeny.
Both start at `E_vk × E_sig` and end at the same surface `A`. The kernel
of `Φ₁` is `K`. The first `e − 1` steps of the chain for `K` are
`Φ₂ ∘ ι`, and the last step is `Ψ`. `ι` fixes every `(X, 0)`, so the image
of `([s]X, 0)` at step `e − 1` is `Φ₂([s]X, 0)`, and the image of
`([2^{e−1}]X, 0)` at step `e` is `Ψ` applied to that.

`A` is the midpoint of an odd-exponent endomorphism. In general it is the
Jacobian of a genus-2 curve, not a product of elliptic curves. The chain
therefore does not split. The two readings are theta coordinates, each in
the theta structure of its own step.

### The two structures, from the literature (2026-11-08)

Read against Dartois, ePrint 2024/1180 (arXiv 2407.15492v2), §2.7, §3.2,
§4.1, §5.3, Remark 18, and Appendix C.2, and against
`Pierrick-Dartois/Theta_dim4` `KaniEndoHalf` and `base_change_theta_dim2`
and `Pierrick-Dartois/SQISignHD-lib` `Verification/Verify.py`, both
Apache-2.0 at the head of their default branches. SQIsignHD (ePrint
2023/436) §5.4–5.7 and Appendices A.4 and C.2–C.3 are cited by those
sources; the ePrint PDF did not yield to retrieval here, so the lemma
text of 2023/436 is taken from Dartois's quotations and from the code.

A level-2 theta structure is induced by a symplectic basis of the
4-torsion (2024/1180 §2.7, Theorem 5(ii)). Along one chain the later
steps inherit that structure: once the first step is set so that its
kernel is the second half of the decomposition, each later step is the
image of the previous kernel, and no fresh change of basis is computed
(§4.1). That inheritance needs an isotropic `K″ ⊂ A[2^{e+2}]` with
`[4]K″` the kernel of the whole chain. These curves have rational
`2^e`-torsion and no further 2-power. The last two steps are therefore
the square-root steps. Remark 18 says a codomain null point computed
from a kernel basis of order 2, without the 8-torsion above it, takes
`2^g − 1` square roots, and `g(g+1)/2` of the resulting signs are
arbitrary. In dimension 2 that is three signs. Those signs are the
unidentified transformation between the step-`e − 1` structure and the
step-`e` structure.

The method that names the transformation explicitly reads a matrix
`M ∈ Sp_4(Z/4Z)` from the image of a symplectic basis of `A[4]`, then
applies the theta transformation formula (2024/1180 Theorem 12;
`base_change_theta_dim2`). The points of `A` this chain can name are
images of `(E_vk × E_sig)[2^e]`. At step `e − 1` the kernel so far is
`(Z/2^{e−1})^2`, so the image is `(Z/2^e)^2 × (Z/2)^2`. Its elements of
order 4 span a `(Z/4)^2`, not a symplectic basis of `A[4] ≅ (Z/4)^4`.
The formula cannot be evaluated.

The method that makes two structures agree by construction is the
half-torsion cutting of §5.3: two chains `F₁` and `F̃₂` from the product,
each of length at most `⌈e/2⌉`, started from symplectic bases of the
`2^{⌈e/2⌉+2}`-torsion chosen so the induced structures on the common
codomain are dual (Appendix C.2, Lemma 29). The check is equality of
theta-null points after one Hadamard transform, and the second chain is
then replaced by its dual (`Verify.py`: `C1.zero() == C2.hadamard().zero()`,
then `F₂ = F̃₂.dual()`). These curves do have `2^{⌈e/2⌉+2}`-torsion. That
construction identifies two chains that both start at the product. It
does not name the transformation between step `e − 1` and step `e` of
the single chain Algorithm 3 prints, and it is not implemented here.

Composing back through `Φ̂₂` meets the same gap at `A`. The return chain
starts in whichever structure step `e − 1` produced, and the dual
identification in the code is the one Lemma 29 arranged in advance.
`Unidentified` stays the result whenever both readings exist. No
identification code is added.

The image check in `Verify.py` accepts each factor up to sign
(`FT[0] == ±a₁P` and `FT[1] == ±a₂P`). The argument named for that is
SQIsignHD Lemma 5.7.1, proved in its Appendix A.4, which was not in the
text retrieved. This verifier does not adopt that argument. The
comparison is not performed.

salt-PRISM does split. Its chain requires a product codomain because it
embeds a full isogeny. `splitting_compute` is that step, and the
fixed-degree known-answer test still runs it. Algorithm 3 does not.

### The basis

On a public key, `R` is the point the basis index names, before
multiplication by `f`. `T` is the point whose affine x-coordinate is the
smallest positive integer at which both points have order exactly `p+1`,
their images in `E[2^e]` generate that subgroup, and their images in
`E[f]` generate that subgroup. Order exactly `p+1` is `[p+1]X = O` and
`[(p+1)/ℓ]X ≠ O` for `ℓ = 2` and every prime divisor of `f`. Two points of
order `2^e` generate `E[2^e] ≅ (Z/2^e)^2` when their `[2^{e−1}]` images are
distinct and nonzero: those images are two of the three nonzero vectors of
`(Z/2)^2`. Two points generate `E[f]` when their images generate each
`(Z/ℓ)^2` for the prime factors of `f`; a scalar and its negative share an
x-coordinate, so the multiples run only to `ℓ/2`. The search stops at `2λ`.

On `E0` every point with an abscissa in `F_p` has the same order-2 image,
`(0, 0)`, the only order-2 point fixed by Frobenius. `R` is still the
smallest positive integer abscissa of order `p+1`. `T` is the least
`a + b·i` with `0 ≤ a ≤ 2λ` and `1 ≤ b ≤ 2λ`, ordered by `a` then `b`,
that completes the same checks.

### Sign

The kernel in the paper is `⟨(P_vk, −P_sig), (Q_vk, −Q_sig)⟩`. The chain
is given x-only points, and `x(−P) = x(P)`, so that kernel and
`⟨(P_vk, P_sig), (Q_vk, Q_sig)⟩` are the same input. They differ by
`ι(X, Y) = (X, −Y)`, and `ι` fixes every `(X, 0)`. The six evaluated
points do not see the substitution.

The comparison, once the two structures are identified, is on `R`, `T`
and `R−T`. A relative sign flip sends `R−T` to `R+T`, and
`x(R+T) ≠ x(R−T)` when both points have order greater than 2. A sign
per factor is the remainder question 4 names, and it is not adopted.

`E0 × E0` is not the bench's general-key number. The curve `j = 1728` has
extra automorphisms. The floor-device gate is the level III chain above,
after a vector has passed the comparison. A fully general end-to-end
timing waits for a §5 signer.

## What `verify` decides today

`verify` is a pure function of the public key, the message, and the
signature. It refuses cheapest-first, and an input that fails more than one
check is refused with the earliest of them:

| Order | Refusal | Cost |
| --- | --- | --- |
| 1 | `Length` | two integer comparisons, on both inputs before either decode |
| 2 | `Canonical` | six field decodes |
| 3 | `Origin` | a zero test on `A` |
| 4 | `Singular` | one `F_{p^2}` multiplication |
| 5 | `Sign` | two encodings and a byte comparison on an element already decoded |
| 6 | `Basis` | up to `2λ` Legendre symbols — 256, 384, 512 — a length the sender chooses with `A` |
| 7 | `Composite` | one cSHAKE256 squeeze and one Baillie–PSW test — more than everything above it together |
| 8 | `Kernel` | the chain, or the discovery that a signature point is off its curve |
| 9 | `Unidentified` | both theta readings exist, and the structures are not yet matched |

The order of `Sign` and `Basis` is load-bearing: the basis search is the one
public check whose length a sender controls, so the free comparison on
`y(P_sig)` runs first and a junk signature never pays for the search. The
`VerifyError` variants are declared in this order and compare in it.

`Kernel` is a chain that stops before both readings, or a signature point
off its curve. `Unidentified` is both readings, with the comparison not
yet decided. The function does not return success. The prepared sample
used by the bench carries canonical zeros for the signature points, which
are not on `E_sig`, so that run stops at `Kernel` and does not time the
chain.
