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
  square in `F_p`. Accept the first `b` for which
  `x(P) = −A / (1 + i·b)` is a non-square on the curve.
- When `A` is not a square: `n` from 1 upward. Accept the first `n` for
  which `x(P) = n·A` is on the curve.

The search stops at 127. No index in that range is a fallback. If none
qualifies, the key is refused (`Basis`) and key generation regenerates it.
The byte the C function returns is `(index << 1)` with the low bit set when
`A` is a square. It is not stored in the key. The order-`2^e` test of the
resulting points is part of Algorithm 3 and is not yet run.

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

Step 3 is specified and not yet the accept condition. The known-answer
source for that chain is the C tree's `theta_chain_compute_and_eval_verify`,
compared at the function level: the same kernel inputs, the codomain, and
the evaluated images, with the chain length as a parameter. The chain
function is shared. The salt-PRISM wrapper around it is not what this
verifier conforms to. Until a vector from that function matches, `verify`
returns `Relation` after the public checks and does not return success.

Table 5's 23.0 Mcycles is salt-PRISM. There is no published timing for §5.
The floor-device gate is a level III run of Algorithm 3 at `e` steps. It is
not run before that chain exists.

## Challenge index

Ruled 2026-11-07. The challenge degree is found by a counter the signer
searches and the signature carries. The verifier tests exactly one `q`.

- `s_j = cSHAKE256(N = "", S = "shekyl-prism-challenge-v1"; public_key_identity ‖ message ‖ LE16(j))`,
  read as `e−1` bits little-endian, low bit forced to 1, bits at and above
  `e−1` cleared. The customization string stays `-v1`: nothing has consumed it.
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

## What `verify` decides today

`verify` is a pure function of the public key, the message, and the
signature. It refuses cheapest-first:

- `Length`, then `Canonical`, then `Origin`, then `Singular`, then
  `Basis`, then `Sign`
- `Composite` when the one derived `q_j` fails Baillie–PSW
- `Relation` when every check above passed

`Relation` is the fail-closed stand-in for Algorithm 3 steps 3–8. It is not
an authorization decision. Nothing in this crate returns success for a
signature.
