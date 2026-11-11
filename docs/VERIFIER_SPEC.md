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
| Instantiated chain length | 248 | 376 | 500 |
| salt-PRISM `a` | 192 | 256 | 320 |
| salt-PRISM signature | 191 bytes | 272 bytes | 350 bytes |
| Table 5 verify, Intel Core Ultra 7 | 9.4 Mcycles | 23.0 Mcycles | 42.2 Mcycles |

A level is a type. A level I encoding is refused by the level V decoder.

### Larger primes — SUPERSEDED 2026-11-09

Checked the same day, and not the modulus. Growing `p` to buy
`2^{e+2}`-torsion costs a limb at NIST I and NIST III. `fp2` takes one
64-bit limb per 64 bits of the prime, so a multiplication grows by about
`(5/4)^2 − 1 ≈ 56%` at NIST I and `(7/6)^2 − 1 ≈ 36%` at NIST III. That is the
wrong direction for the floor-device gate, and it retires the C chain
vectors. `f·2^{e+2} − 1` at Table 2's `f` is composite besides (question
4). The windows that keep today's 4, 6, and 8 limbs are 249–256, 377–384,
and 505–512 bits, and no prime `f·2^v − 1` with `f` odd and `v ≥ e+2`
sits in the NIST I or NIST III window.

| | Prime | Bits | Limbs | Status |
| --- | --- | --- | --- | --- |
| NIST I | `43·2^{251} − 1` | 257 | 5 | SUPERSEDED |
| NIST III | `171·2^{378} − 1` | 386 | 7 | SUPERSEDED |
| NIST V | `33·2^{503} − 1` | 509 | 8 | SUPERSEDED |

### Chain exponent — DECIDED 2026-11-09, chain built, identification not

Table 2 is salt-PRISM's table. §5 writes its torsion exponent as `e` and
prints no parameter row of its own, so taking the §5 chain length to be
Table 2's `e` was this verifier's inference. The corrected proposal keeps
the printed primes and sets the §5 exponent to `e′ = e − 2` (246, 374,
498).

`Φ₁` is a `(2^{e′}, 2^{e′})`-chain and `Φ₂` a `(2^{e′−1}, 2^{e′−1})`-chain.
A chain of length `len` in the SQIsignHD construction needs rational
`2^{len+2}`-torsion (Dartois 2024/1180 §4.1, Lemma 29). For `Φ₁` that is
`2^{e′+2} = 2^e`, which these curves have exactly. For `Φ₂` it is
`2^{e−1}`, and `2^e` supplies one further doubling. The codomain check is
equality of theta-null points after one Hadamard transform, with the dual
structures arranged before either chain starts.

`q = 2^{2e′−1} − s² = 2^{2e−5} − s²`, with `s` odd and `0 < s < 2^{e′−1}`.
That is §5's formula with `e′` in place of its exponent. The degree loses
four bits against `2^{2e−1}` and stays far above `2^{2λ}` (491, 747, and
995 bits against 256, 384, and 512). The paper's norm condition, read at
`e′`, is `N² > 2^{2e′} + 1`. With `N = p + 1 = f·2^e`, the ratio
`N² / 2^{2e′}` is `f²·2^4`, at least `25·16 = 400` since `f ≥ 5`. And
`s < 2^{e′−1}` still gives `4s² < 2^{2e′}`.

The signer keeps the full `2^e`-torsion for IdealToIsogeny (§7.1) and
evaluates `σ` on `E_vk[2^{e′}]`. The field, the limb counts, the key
size, the signature length, and the C chain vectors stay as they are.
The signature does not carry `s`. The derived challenge scalar is `e − 3`
bits wide, which is `e′ − 1`. At Table 2's three levels that is the same
byte length as the `e − 1` width.

The instantiated verifier walks a chain of length `e′`. The kernel is the
canonical basis of `E_vk[2^e]`, with `x(Q) = −x(P) − A`, doubled twice to
order `2^{e′}`. Levels I and III of the §5 vectors reach both readings, so
the result is `Unidentified`. `(P, −Q)` stops at isogeny step 1. Level V's
honest vector stops at isogeny step 1, and `(P, −Q)` is the one that
reaches both readings. The cause of that relative sign is open. Level V's
index point is still not of order `p + 1`; the verify path does not consult
that search. The dual-basis identification is not built. The
Nakagawa–Onuki recheck below cleared the degrees: no `q` in the `e′`
window meets the small-extension bound.

A third reading for the authors — whether the §5 chain was meant to run
two steps shorter than the rational 2-torsion — is held. It goes out with
the follow-up if question 4(a) and 4(b) come back unanswered. An answer
to either makes it moot, so it is not part of the letter now.

### Small-extension torsion — CLEARED 2026-11-09

ePrint 2025/1602 ([53]) attacks PRISM-id when the challenge prime `q`
divides `(−p)^k − 1` for a small positive `k`: the `q`-torsion is then
rational over `F_{p^{2k}}`. Lemma 3, and the general case in its
Appendix A, computes a `q`-isogeny in `Õ(max{k², k√q})` base-field
operations once such a `k` is known. Section 5.3 of that paper takes a
bit length `a = 4λ/3` to hold every variant of the attack at or above
`2^λ`, and `a = 7λ/6` to hold the variant that may reject challenges
(Theorem 2, cost `Õ(2^{6a/7})`). The paper says the attacks do not
affect PRISM-sig.

At `e′` the degree is `q = 2^{2e−5} − s^2`. Because `s < 2^{e′−1}`,
every such `q` lies in `[2^{2e−6}, 2^{2e−5})` and has bit length `2e−5`:
491, 747, and 995. That is above both of [53]'s bit-length bounds
(`7λ/6` is 149, 224, and 299). It is also enough by itself.
`√q ≥ 2^{e−3}` is `2^245`, `2^373`, and `2^497`, and each of those is
larger than `2^λ`. Lemma 3's cost is therefore above `2^λ` for every
positive `k` and every `q` in the window. Moving the window from
`2^{2e−1}` to `2^{2e−5}` does not produce a degree the attack can use.
The index search does not gain a `q`-filter.

`p = f·2^e − 1` is a special prime, so the sample checks that `−p` does
not have a small order modulo these degrees. `scripts/sample_small_extension.py`
draws the first 1,000 prime `q_j` at exponent `e − 2` for each level,
through `sample_e_prime` and the verifier's own derivation, over
deterministic keys and the message prefix `shekyl-prism-small-extension`.
The order of `−p` modulo `q` was above `2^24` for all 3,000 degrees, and
for the three committed §5 degrees as well. The median smallest index
was 63, 100, and 139. The largest was 692, 940, and 1343.

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
which is where the C's own fallback lives. The order-`p+1` test of the
resulting point runs in the relation. A point that is on the curve and
not of that order is `Kernel` before the chain. The level V §5 vector is
that case.

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
2. Let `q = 2^{2e′−1} − s^2`. Refuse unless `q` is prime. The verifier tests
   exactly one `q`, the one named by the carried index. The paper prints
   `2^{2e−1}`.
3. The isogeny check is Algorithm 3 steps 3–8: a `(2^{e′}, 2^{e′})` chain on
   `E_vk × E_sig` in the theta model (gluing from the product, `(2, 2)`
   steps, splitting, point evaluation), a basis of `E_vk[p+1]` pushed
   through `Φ`, and acceptance only when `[2^{e′−1}] Ψ(R') = [s] R'` and
   the same holds for `T'`. The instantiated chain length is `e′`. The
   paper prints `e` for that exponent and prints no row, so equating it
   with Table 2 was this verifier's inference. The comparison itself is
   not built. salt-PRISM's `a` is a third count.

Step 3 is the accept condition. `chain` matches
`theta_chain_compute_and_eval` on two length-160 kernels, each the graph of
an endomorphism of `E0`: one with the extra 8-torsion the reference's
verify path passes, and one without it, which is the path Algorithm 3 uses
because a rational point has order dividing `2^e` and not `2^{e+2}`. Both
were run again with the isotropy checks on, and the codomain and the image
still match. The vectors are `prism-isogeny/testdata/chain_lvl1.txt` and
`chain_lvl1_order.txt`. The salt-PRISM wrapper around that function is not
what this verifier conforms to.

`verify` then runs one chain of length `e′` on
`K = ⟨(P_vk, −P_sig), (Q_vk, −Q_sig)⟩`, and does not split the codomain.
The chain is evaluated at `([s]X, 0)` and `([2^{e′−1}]X, 0)` for `X` in
the canonical basis of `E_vk[2^e]`. The images of `([s]X, 0)` would be read
at step `e′ − 1`, and the images of `([2^{e′−1}]X, 0)` at step `e′`. Levels
I and III reach both readings. Level V's honest vector stops at isogeny
step 1. A signature point that is not on `E_sig`, or a chain that stops before both
readings, is `Kernel`. Both readings existing is `Unidentified`: the
transformation between the structures is open, so the comparison is not
decided and the function does not return success. The degree-`q` witness in
`prism-verify/testdata/e0_endomorphisms.txt` does not reach that refusal.
It is the graph of one endomorphism of `E0` (`j = 1728`), built in the C
tree by `endomorphism_application_even_basis` on `BASIS_EVEN` of
`CURVE_E0` and pushed by `theta_chain_compute_and_eval` with the extra
torsion off. The same kernel in the Rust port dies at isogeny step 3,
where a theta coordinate of the step's kernel is zero. The reference
calls that vanishing an unexpected split. Both readings are not produced,
so the result is `Kernel`. The stop is the starting curve: a random
`E_vk` meets a product, or a vanishing coordinate, about `1/p` of the
time at each step.

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
  read little-endian, low bit forced to 1, bits at and above `e′ − 1 = e − 3`
  cleared. The byte length at Table 2's three levels is still `⌈(e−1)/8⌉`,
  because `e − 1` and `e − 3` fall in the same byte. The top byte keeps
  `((e − 4) mod 8) + 1` bits, which is `1..=8`: a width that is a multiple of
  eight keeps the whole byte, and no width keeps none of it. The
  customization string stays `-v1`: nothing has consumed it.
- The public key's identity is the byte string the key decoder accepted, at
  `PUBLIC_KEY_LEN` bytes. The signer and the verifier reach the hash through
  the same decoder, so neither can absorb a different string.
- `q_j = 2^{2e′−1} − s_j^2 = 2^{2e−5} − s_j^2`.
- The signer takes the smallest `j` in `0..=65535` for which `q_j` passes
  Baillie–PSW, so signing is deterministic for a given key and message.
  Exhausting the range is a signing error, not a fallback.
- The signature carries `j` as 2 bytes little-endian. It does not carry `s`.
  The verifier derives `s_j`.
- The verifier does not check that `j` is the smallest. It derives `s_j`,
  tests `q_j` once, and refuses on composite.

An intermediate surface of an honest chain can still split, or a theta
coordinate can vanish. On a random key that is about `1/p` at each step,
so over a chain of length `e′` the chance is about `e′/p`. The signer
moves to the next index. The index that is carried is the smallest `j`
whose `q_j` is prime and whose chain completes, which keeps signing
deterministic. This verifier does not yet apply the second half: it
does not skip an index whose chain fails to complete. The carried index
of a vector here is the smallest prime index. One vector per level is
`prism-verify/testdata/s5_lvl{1,3,5}.txt`, produced by
`scripts/c/export_s5_vectors.c` from the pinned tree's
`quat_sampling_random_ideal_O0_given_norm`, `quat_lideal_inter`, and
`dim2id2iso_arbitrary_isogeny_evaluation`, with `t = s^{-1} mod 2^{e′}`
and the image doubled down to `E[2^{e′}]`. The `q` line of each file is
one degree at `e′`. The small-extension sample is the thousand degrees
per level in Small-extension torsion, not these three lines. The relation derives `s_j` at `e′` and runs the length-`e′` chain on the
canonical basis. Levels I and III reach both readings (`Unidentified`).
Level V stops at isogeny step 1 (`Kernel`).

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
4. Two parts, in the letter. (a) Did §5 intend `p = f·2^{e+2} − 1`?
   With Table 2's `f` that integer is composite at all three levels:
   NIST III is divisible by 911, NIST V by 49954549, and NIST I is
   composite by the Baillie–PSW test. (b) Is there an identification of
   the step-`e − 1` and step-`e` readings that uses only the rational
   `2^e`-torsion? The reading under Identification found none in the
   published method. A third reading, whether the chain was meant to run
   at `e − 2`, is held for the follow-up (see Chain exponent).
5. Deterministic Baillie–PSW is the test this verifier runs. ePrint
   2026/906 shows that a weak primality test breaks the standard-model
   assumption, and that random-base Miller–Rabin suffices in the random
   oracle model. The signer chooses the message and the index, so it can
   grind for a pseudoprime, and two consensus verifiers cannot draw bases
   without risking a split. Whether the authors accept deterministic
   Baillie–PSW against that signer, Baillie–PSW plus further fixed-parameter
   Lucas rounds, or a signer-supplied certificate, goes in the follow-up.
   No second test is added before they answer.

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

The published route that identifies the codomain is two chains from the
product, `Φ₁` and `Φ₂`, with the dual structures of Lemma 29 arranged
before either starts. The basis conditions that arrangement has to meet
are the next section. Each chain needs rational `2^{len+2}`-torsion. On
Table 2 that holds when the lengths are `e′ = e − 2` and `e′ − 1`, which
is the chain-exponent decision above. The two chains are not built. The
single chain at `e′` stops before both readings, so its result is
`Kernel`. `Unidentified` is the result when both readings exist.

### Dual structures at `e′` — PLANNED 2026-11-10

Not built. This is the construction the identification code has to meet.
Nothing in the tree computes these bases yet. The live chain is the
single chain at `e′`, and it stops before both readings.
The factors are the ones Identification names, read at exponent `e′`
rather than `e`. Dartois writes `F̃₂` for our `Φ₂` and `F₂` for our
`Φ̂₂`. The names `domain` and `codomain` are local to this section:
`domain` is the product `E_vk × E_sig`, and `codomain` is the common
middle of the two chains. Neither is the Montgomery coefficient, and
neither is the midpoint the length-`e` chain calls `A`.

The endomorphism is §5's, read at exponent `e′`. Dartois's equation (26)
is `ι ∘ Φ ∘ ι` for `ι(X, Y) = (X, −Y)`: the off-diagonal signs are
opposite, and that map is not the one Algorithm 3 factors.

```text
Φ(X, Y) = ([s]X − σ̂(Y), σ(X) + [s]Y)
```

`s² + q = 2^{2e′−1}` and `t = s^{-1} mod 2^{e′}`, so
`q ≡ −s² (mod 2^{e′})` and `tq ≡ −s (mod 2^{e′})`. The response `σ` is
defined from the normalized signature points, the normalization having
already chosen the sign: `P_sig = [t]σ(P_vk)` and
`Q_sig = [t]σ(Q_vk)`. Then `σ(P_vk) = [s]P_sig` and
`σ̂(P_sig) = [tq]P_vk = −[s]P_vk`, and the same for `Q`. On
`domain[2^{e′}]`,

```text
σ(a P_vk + b Q_vk) = [s](a P_sig + b Q_sig)
σ̂(a P_sig + b Q_sig) = −[s](a P_vk + b Q_vk)
```

`Φ(P_vk, −P_sig) = ([s]P_vk + σ̂(P_sig), σ(P_vk) − [st]σ(P_vk)) = (0, 0)`,
and the same for `Q`. These points have order `2^{e′}`, so

```text
ker(Φ₁) = ⟨(P_vk, −P_sig), (Q_vk, −Q_sig)⟩
```

`Φ₁` has length `e′`. This is the negative-pair kernel the length-`e`
chain already takes, at the shorter order.

Doubling the long kernel gives the negative pairs of order `2^{e′−1}`.
Algorithm 3 line 4 builds its first chain on the positive doubles, which
is `ι` of that subgroup and not the subgroup itself:

```text
ker(Φ₂) = ι([2]ker Φ₁) = ⟨([2]P_vk, [2]P_sig), ([2]Q_vk, [2]Q_sig)⟩
```

`Φ₂` has length `e′ − 1`. Both chains start at the product. Both kernels
are full points. The length-`e` chain takes the negative pairs as x-only
input, and that input cannot tell the sign; here the sign is part of the
kernel. Step 8 still does not see `ι`: it is given `(X, 0)`, and `ι`
fixes every such point.

Lemma 29 is Dartois, ePrint 2024/1180, Appendix C.2. He cites it as
SQIsignHD Corollary 58 (ePrint 2023/436, Appendix F.3). The statements
differ, and this section follows Lemma 29. Corollary 58 asks for one
theta structure, with `F(c₂ y′) = c₂ v′` and no sign. Lemma 29 asks for
dual structures, related by

```text
J = |  0  −I₂ |  ∈ Sp₄(Z/4Z),
    |  I₂   0 |
```

so one Hadamard takes one theta-null point to the other. Dartois writes
`I₄` and `Sp₈` for `E₁² × E₂²`. The product of two elliptic curves is
that statement with two points. The codomain check is the Hadamard
equality. Step 8 is the composition `Φ̂₂ ∘ Φ₁`.

Each factor has rational `2^e`-torsion, and `2^e = 2^{e′+2}`. A chain of
length `len` needs a symplectic basis of `domain[2^{len+2}]`. The two
lengths and Dartois's kernel multiples are one family:

```text
len₁ = e′      c₁ = 2^{e − len₁} = 2^2 = 4
len₂ = e′ − 1  c₂ = 2^{e − len₂} = 2^3 = 8
```

Dartois writes `c_i = 2^{e′+2−e_i}` for a basis of the `2^{e′+2}`-torsion.
Here that exponent is our `e`, and `e_i` is the chain length. Let
`(X₁, X₂, Y₁, Y₂)` and `(U₁, U₂, V₁, V₂)` be `ζ`-symplectic
bases of `domain[2^e]`, for one primitive `2^e`-th root `ζ`, with

1. `ker(Φ₁) = [c₁]⟨Y₁, Y₂⟩` and `ker(Φ₂) = [c₂]⟨V₁, V₂⟩`.
2. `[4]V_l = −[4]Φ(X_l)` and `[4]Y_l = [4]Φ̂(U_l)` for `l ∈ {1, 2}`.

`[4]X_l` and `[4]U_l` lie in `domain[2^{e′}]`, which is the subgroup the
signature determines. The signature does not carry `σ` of
`E_vk[2^e]`. The order-`2^e` points are the rational torsion on each
factor. Condition 2 is how those points meet the signature.

Lemma 29 prints `[c₂]Φ(X_l) = −[c₂]V_l` and `[c₁]Φ̂(U_l) = [c₁]Y_l`.
At these lengths that is `[8]Φ(X_l) = −[8]V_l` and
`[4]Φ̂(U_l) = [4]Y_l`. The first of those leaves a shear. Replacing `V₁`
by `V₁ + [2^{e′−1}]U₁` keeps the symplectic form, both kernels, and both
printed projections, because `[8][2^{e′−1}] = [2^{e′+2}] = 0` on this
torsion. The same replacement adds `[2^{e′}]Φ₂(U₁)` to the order-4 image
below, and that point is the complement component of `B₂`. The equation
`[4]V_l = −[4]Φ(X_l)` rejects the shear:
`[4][2^{e′−1}]U₁ = [2^{e′+1}]U₁` has order 2.

The codomain bases are bases of `codomain[4]`. Multiplying the kernel
half by `c_i` does not give one. `[c_i]Y_l` is in the kernel, so
`[c_i]Φ₁(Y_l) = 0`, and likewise `[c₂]Φ₂(V_l) = 0`. The multiple that
lands on order 4 is `c_i / 4`, which is `1` on the long chain and `2` on
the short chain:

```text
B₁ = ([2^{e′}]Φ₁(X₁), [2^{e′}]Φ₁(X₂), Φ₁(Y₁), Φ₁(Y₂))
B₂ = ([2^{e′}]Φ₂(U₁), [2^{e′}]Φ₂(U₂), [2]Φ₂(V₁), [2]Φ₂(V₂))
```

Lemma 29's relation is `B₂ = J B₁`. The complement coefficient
`2^{e′}` is his, for both chains: it is the available exponent, not the
chain length.

`[4]V_l = −[4]Φ(X_l)` fixes `V_l` modulo `domain[4]`. A difference of
order 4 need not die under `[2]Φ₂`, so that congruence does not by
itself make `[2]Φ₂(V_l)` one point. The same holds for `Y_l` and
`Φ₁(Y_l)`. The symplectic completion is the three block equations of
`complete_symplectic_matrix_dim2`, over `Z/2^e Z`, with the kernel in
the right-hand blocks `(C, D)` and the complement in `(G, H)`:

```text
HᵀG = GᵀH,    CᵀD = DᵀC,    GᵀD − HᵀC = I₂
```

Those equations complete a known half. They do not choose the
`domain[4]` lift. The lift is part of building the bases. It is not a
search through `Sp₄(Z/4Z)` after the bases exist, and it is not settled
by taking any point of order `2^e` with the right image under `[8]`.
`Unidentified` stays until the bases that were built satisfy
`B₂ = J B₁`.

The product theta structure comes first. Take a basis `(P₁, P₂)` of
`E_vk[2^e]` and points `(R₁, R₂)` of `E_sig[2^e]` whose `[4]` images are
the response basis. The Weil pairing scales by `q`, so the product basis
is symplectic once `R₂` is multiplied by `q_inverse = q^{-1} mod 2^e`:
`e(P₁, P₂) = e(R₁, [q_inverse]R₂)`. Doubling `e′` times lands on order
4. Each order-4 basis is then replaced by a canonical symplectic basis
that keeps the pairing. The product theta structure is the one for

```text
((T₁, 0), (0, U₁), (T₂, 0), (0, U₂)).
```

`q_inverse` reduced modulo 4 is the scalar in
`base_change_canonical_dim2`. The basis of `domain[2^e]` uses the
inverse modulo `2^e`.

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
points do not see the substitution. The dual-structure bases do. They
are full points. `Φ₁` takes the negative pairs, and `Φ₂` takes
`ι` of their doubles, the positive pairs
`([2]P_vk, [2]P_sig)`, `([2]Q_vk, [2]Q_sig)`, before step 8 runs.

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
