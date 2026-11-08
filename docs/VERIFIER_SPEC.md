# PRISM verifier spec (draft)

Status: draft, written to be taken to the authors. This file is not a
standard. Numbers below are copied from the working copy
`shekyl-dev/docs/papers/2026-PRISM_signature.pdf` (ePrint 2026/443, Table 2
and §5 Algorithm 3). The SQIsign parameter source is
`shekyl-dev/docs/papers/SQIsign-20250205.pdf`. The ECLIPSE PDF (ePrint
2026/2312) was not retrieved on 2026-11-07 (the ePrint host returned 403 and
`ia.cr` returned 404); the two open questions are recorded from the lane
brief and stay open until that copy is in `docs/papers/`.

The candidate authorization level, if a later design round adopts one, is
NIST III. NIST I and NIST V are implemented so the same binary can time them.

## Parameter rows

Table 2. `p = f·2^e − 1`. Each `p ≡ 3 (mod 4)`.

| | NIST I | NIST III | NIST V |
| --- | --- | --- | --- |
| Prime | `5·2^248 − 1` | `65·2^376 − 1` | `27·2^500 − 1` |
| `e` | 248 | 376 | 500 |
| `λ` | 128 | 192 | 256 |
| Public key | 66 bytes | 98 bytes | 130 bytes |
| salt-PRISM `a` | 192 | 256 | 320 |
| salt-PRISM signature | 191 bytes | 272 bytes | 350 bytes |
| Table 5 verify, Intel Core Ultra 7 | 9.4 Mcycles | 23.0 Mcycles | 42.2 Mcycles |

A level is a type. A level I encoding is refused by the level V decoder.

The public key is the Montgomery coefficient `A` of `E : y^2 = x^3 + A x^2 + x`
over `F_{p^2}`, little-endian canonical `fp2` encoding (`(bit length + 7) / 8`
bytes for each of the two `F_p` components), followed by the 2-byte basis
hint from §7.2. The hint is carried and is not consumed. `A^2 ≠ 4`, so the
curve is nonsingular. A non-canonical field element is a refusal.

## Variant

The variant this verifier will accept, once Algorithm 3's image check is
complete, is §5, not salt-PRISM (§6). salt-PRISM is EUF-CMA. §5 aims at
strong unforgeability because a malleable signature would make a malleable
transaction id. The C reference implements salt-PRISM only.

Algorithm 3 (`VerIsogenyStrong`) takes `E_vk`, `(E_sig, P_sig, Q_sig)`, and
an odd integer `s` with `0 < s < 2^{e-1}`:

1. Refuse unless `y(P_sig)` is strictly less than `y(−P_sig)` in the
   little-endian encoding order. `y = 0` is its own negative and is kept.
2. Let `q = 2^{2e−1} − s^2`. Refuse unless `q` is prime.
3. The isogeny check is Algorithm 3 steps 4–8: build `Φ` and `Ψ`, push a
   basis of `E_vk[p+1]`, and accept only when
   `[2^{e−1}] Ψ(R') = [s] R'` and the same holds for `T'`.

Step 3 is specified and not yet the accept condition. The engine evaluates
split (2,2)-isogenies (a pair of Vélu 2-isogenies) and a chain of them of
length `a`. The C function `theta_chain_compute_and_eval_verify` is the
glued chain those steps have to meet. Until a known-answer vector from that
function matches, `verify` returns `Relation` after the public checks, and
does not return success. A refusal is never turned into a fresh salt.

## Salt, hash, primality

§5's `s` is not salt-PRISM's salt. When a salt is used (§6), the paper sets
`k = 2λ` bits. This decoder does not accept a `uint32_t` salt. The C
reference stores the salt in a `uint32_t`, absorbs
`sizeof(uint32_t)` native-endian bytes in `hash_to_challenge`, and when the
value is `0xFFFFFFFF` draws a fresh salt inside the verifier. That is a
records-was quirk of `external/PRISM_v2` at
`87fad7da5f12f79112521677f4f5530672afe9ba`. Our verifier has no randomness
argument.

The challenge hash is cSHAKE256 (SP 800-185), function name empty,
customization `shekyl-prism-challenge-v1`. The input is the public-key bytes
then the message. The output is `e−1` bits, little-endian, with the low bit
forced to 1 so `s` is odd without resampling. `s` in the signature must
equal that integer. `q` is then tested once.

Primality is Baillie–PSW: trial division by the primes up to 100, one strong
Miller–Rabin test at base 2, then the strong Lucas test with Selfridge's
parameters (`D` from `5, −7, 9, −11, …` until the Jacobi symbol is −1,
`P = 1`, `Q = (1−D)/4`). No random bases. `ibz_probab_prime(n, 20)` in the
C tree is not this test. Accepting a composite `q` would let the isogeny
factor into small degrees.

## Encoding of the §5 research signature

This layout is the one this draft freezes. It is not Table 2's salt-PRISM
size, and it is not the C reference's 260/388/516-byte encoding. A signature
of those lengths is a refusal.

| Field | Contents |
| --- | --- |
| `A_sig` | Montgomery coefficient of `E_sig`, one `F_{p^2}` element |
| `P_sig` | affine `x`, `y`, each one `F_{p^2}` element |
| `Q_sig` | affine `x`, `y`, each one `F_{p^2}` element |
| `s` | `e−1` bits, little-endian, width `ceil((e−1)/8)` |

NIST I is 351 bytes, NIST III is 527 bytes, NIST V is 703 bytes. Wrong
length, non-canonical element, even `s`, `s` out of range, or `s` not equal
to the challenge hash is a refusal.

## Open questions

Not built, and not decided:

- A re-randomized per-output key is unlinkable to the long-term key.
- Re-randomization holds for this §5 variant. ECLIPSE claims it for a
  prime-degree variant, and its strong-unforgeability proof is still open.

## What `verify` decides today

`verify` is a pure function of the public key, the message, and the
signature. It returns:

- `Length`, `Canonical`, `Singular`, `Sign`, `Salt`, `Composite` for the
  checks above
- `Codomain` when the split chain of length `a`, started at Montgomery
  coefficient 0 with kernel `(0, 0)`, does not produce the signature's
  claimed codomain coefficient — this is an engine check, not Algorithm 3
- `Relation` when every check above passed

`Relation` is the fail-closed stand-in for Algorithm 3 steps 4–8. It is not
an authorization decision. Nothing in this crate returns success for a
signature.
