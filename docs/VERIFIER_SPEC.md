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

`verify` then runs two chains. The longer one has length `e` and kernel
`(P_vk, P_sig)`, `(Q_vk, Q_sig)`. The shorter one has length `e − 1` and
kernel the doubles of those points. The point pushed through the longer
chain is `([2^{e−1}] R, 0)`; through the shorter one, `([s] R, 0)`. `R` is
the point the basis index names before multiplication by `f`, so its order
divides `p + 1`. Acceptance is the two codomains being the same product and
the two images being the same point. A signature point that is not on
`E_sig`, or a chain that does not split, is `Kernel`. Images that differ
are `Image`.

Table 5's 23.0 Mcycles is salt-PRISM. There is no published timing for §5.
The floor-device gate is a level III run of a splitting chain at `e` steps.
The bench sample stops at `Kernel` and is not that run.

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
4. The two comparison questions in "How step 8 compares points", below.
   They are written here so the accept condition has a stated
   identification and a stated sign rule before any vector is allowed to
   turn a chain match into a claim about the paper. Confirmation that those two readings
   match the paper is the authors'.

## How step 8 compares points

These two rulings are how step 8 reads the chain output. The chain
function now matches the reference on the vectors named above; the
comparison below is what `verify` requires of that output.

### Identification

Step 4 builds `Φ` as `e − 1` (2,2)-steps. Step 5 builds `Ψ` as one more
step, so the chain a node evaluates has length `e`. The paper's
factorization `Φ₁ = Ψ_e ∘ Φ₂ ∘ ι` says those two computations land on the
same abelian surface. A theta-model implementation does not land in a
unique coordinate system: each step chooses a theta structure, and two
structures on one surface differ by a signed permutation of the four
theta coordinates.

The comparison is not made in those raw coordinates. The splitting step
records the isomorphism back to an elliptic product — the matrix the C
reference calls `M` and applies with `apply_isomorphism` before
`theta_point_to_montgomery_point`. `splitting_compute` and
`montgomery_couple` are that pair. Both sides of step 8 are read in that
product's Montgomery model, after that one isomorphism. A codomain that
does not split is a refusal, not a second choice of structure. Verification
in the pinned tree also requires the vanishing structure to be index 8;
`splitting_compute` takes that index and refuses any other when the extra
torsion is present. Algorithm 3 does not pass the extra torsion, so it
accepts whichever single structure vanishes. `chain` is
`_theta_chain_compute_impl` with randomisation left out, and `verify` calls
it twice. A basis of each factor is not by itself a kernel: the reference
refuses that product. The kernel that succeeds is the graph of an
endomorphism, which is what the two length-160 vectors are, and what
Algorithm 3 builds from `(P_vk, P_sig)`.

### Sign

A Montgomery `(X : Z)` and a theta point `(x : y : z : t)` are both even:
each identifies a point with its negative. On a product `E1 × E2` the sign
can flip per factor, not only globally, and an x-only equality of the
step-8 images would accept a scalar and its negative on each factor.

The signature carries affine `(x, y)` for `P_sig` and `Q_sig`, so `−P_sig`
and `P_sig − Q_sig` are exact group-law points, not a choice of square
root recovered from an x-coordinate. The kernel points of `Φ` are the
doubles `[2]P_vk`, `[2]P_sig` and their partners. Doubling identifies a
point with its negative in x-only form, so that kernel does not see the
global sign. The step-8 equality does. It is compared as affine points on
the product, with the y-coordinates fixed by the signature's `y` and by
the same group law, not as x-only or theta coordinates. An image whose
y-coordinate is the other square root fails the equality.

`E0 × E0` is not the bench's general-key number. The curve `j = 1728` has
extra automorphisms, and gluing from it can take a path a random product
does not. The length-`e` chain on a C-generated general pair is the number
the floor-device gate reads, plus the verifier's fixed overhead, after a
positive vector has passed. A fully general end-to-end timing waits for a
§5 signer.

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
| 8 | `Kernel` | the chains, or the discovery that a signature point is off its curve |
| 9 | `Image` | the two chains split and their images differ |

The order of `Sign` and `Basis` is load-bearing: the basis search is the one
public check whose length a sender controls, so the free comparison on
`y(P_sig)` runs first and a junk signature never pays for the search. The
`VerifyError` variants are declared in this order and compare in it.

`Kernel` is a chain that does not split, or a signature point off its curve.
`Image` is a split whose two sides disagree. Agreement is `Ok(())`. The
prepared sample used by the bench carries canonical zeros for the signature
points, which are not on `E_sig`, so that run stops at `Kernel` and does
not time a splitting chain.
