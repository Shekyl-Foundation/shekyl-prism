# Signing-intersection oracle (Rick, t45u, question 2)

> **Follow-up (t49u, t51u):** oracle-only fixtures with the column HNF, composite odd key norms accepted by admission, and a corrected comaximal-versus-coprime statement are in [`followup/FOLLOWUP.md`](followup/FOLLOWUP.md). Section 3 below has been corrected accordingly.

Branch `experiment/signing-intersection-oracle`, off `research` (`c94f25b`). Prototype code read and run
from `origin/dev` `470d537` (PR #47, #48, #49 and #50 merged); nothing pushed to any PR, `dev` untouched.

**Question.** Can an independent exact-integer oracle verify the specialized signing intersection (the
native, factor-256 guarded class: `GuardedIdeal::admit` then `GuardedIdeal::intersection`,
`shekyl-prism/src/ideal/mod.rs`) against the general coprime-ideal construction, at all five sets?

## Verdict: **yes, with conditions** (no mismatch found)

The oracle agrees with the prototype on every row, at all five sets. The specialized output is
**exactly** the lattice `I_sec ∩ I_q` that the general construction gives, whenever the conditions in
section 3 hold, and every condition is either checked at runtime (and marks the output when it fails)
or follows from an inequality that I verified in exact integers at all five sets (section 3, table).
The conditions are not caveats about the code; they are the hypotheses of the equality.

| Evidence | Result |
|---|---|
| Structured corpus, 56 rows × 5 sets (guard boundary, order, units, composite / 1 mod 4 / square norms, `N` at the Q4 limit, `q` extremes, bad roots, bad scalars, worst-case width) | **280 / 280** admission and intersection verdicts agree; 39 intersections per set, each equal to all four general routes |
| Random corpus, 150 rows × 5 sets, random `N`, shear inside the guard, units, roots in unreduced / negated forms | **750 / 750** agree; all 750 intersections equal all general routes |
| Prototype's committed fixtures (`vectors/*.txt`, `ideals/*.txt`, 55 + 30 rows) | oracle's own reduction reproduces all 55 reduced bases and normalized forms bit for bit; all 30 committed guarded rows agree through the harness |
| Toy primes `p ∈ {3,7,11,19,23}`, **every** left ideal of prime norm `N ≤ 13` against every generic ideal of norm `q` (`toy.py`) | 1,294 pairs (218 with non-generic `I_sec`), **24,694** specialized outputs, all lattice-equal; 0 mismatches |
| Medium random sweep, `p, N, q` to 2⁵⁰, prime / composite / square / non-generic `N`, `q` of both residues mod 4 (`medium.py`) | 14,932 comaximal pairs, **89,592** outputs, all equal; 0 mismatches |

## 1. The two constructions

**Specialized (as implemented).** Numerator coordinates `X = 2x`, `J` = left multiplication by `i`. The key
stores a Gaussian pre-reduced basis `(u, v)` of `I_sec` (norm `N`), admitted by `GuardedIdeal::admit`
(norm odd, positive, `< 2^B`; `|det_{Z[i]}(u,v)|² = 4N²`; columns of `GenericHnf(N, root)` in the span, in
canonical HNF shape, with product of pivots `4N²`; closure under the `O0` generators; guard
`0 < N4(u) ≤ N4(v)`, `N4(v)² ≤ p(1024N)²`). For the public challenge scalar `s` it forms `q = 2^(2e−5) − s²`
(odd scalar, prime degree by Baillie–PSW, bit length `2e−5`), checks `p + x² + y² ≡ 0 (mod q)` for the
challenge root `r = x + yi`, and computes in `Z[i]/q`

    ρ(a,b,c,d) = (a + bi) − (c + di)·r  (mod q),   γ = −ρ(u)·conj ρ(v) / N(ρ(v)),  centred componentwise,
    U = q·v,   V = u + γ₀ v + γ₁ Jv,   carried norm T = N·q.

The inverse of `N(ρ(v))` is `norm^(q−2)`, and the product-back `inv·norm = 1` is checked. Failure of any
check marks all nine outputs.

**General (oracle).** `I_q` is built as the left-`O0` closure of `{q, r + j}` (all `O0`-basis × generator
products in own quaternion arithmetic, associativity and `O0`-closure tested), `I_sec` as the closure of
`{N, root}`, with no shape assumed. `I_sec ∩ I_q` is then computed four ways, each canonicalized to row-HNF:
(a) Sage `FreeModule.submodule(...).intersection`; (b) the dual-lattice route `(L₁* + L₂*)*` with PARI `mathnf`;
(c) the kernel of `ρ` restricted to `I_sec` by integer linear algebra, which uses no coprimality;
(d) the coprime formula `q·I_sec + N·I_q` (Q3, under `gcd(N,q) = 1`). A fifth check: the lattice of `GenericHnf(Nq, root_CRT)`, the general constructor
at the product norm with the parity-lifted CRT root, equals the specialized lattice. The four routes agreed
with each other on every row, and the specialized lattice `Z-span{U, JU, V, JV}` equals all of them.

**Gram level.** For each intersection: `(N4U, N4V, D(V,U), D(V,JU))` divides exactly by `2T`, the normalized
Gram has determinant `p`, and the exact Gaussian reduction (rules of SIGNER_SPEC 2.3, own code) of the
specialized pair and of the generic pair `(2T,0,0,0),(x,y,1,0)` give the same `(A', C')`. The generic
Gram is `(2T, (x²+y²+p)/(2T), x, y)` and has determinant `p` as well. Active steps (including the final
no-op check) and swap counts of the two starts differ, as expected: the pre-reduced start needs fewer swaps
(for example 158 against 167 at P248). That is only a count observation, not a claim about the schedules.

## 2. Independence

`oracle.py`, `gen_corpus.py`, `gen_random.py`, `check.py`, `toy.py`, `medium.py` import nothing from the prototype, from
`research/*` or from `scripts/stage2c_oracle.py` / `stage2d_fixtures.py`. The only prototype artifact used is the
Rust binary's *output* (`rust_harness/sio_dump.rs` calls `hazmat::ideal` and prints), plus the committed
fixture files as additional inputs. Primes come from Sage (`is_pseudoprime`, the same strength as the
prototype's Baillie–PSW), lattices from Sage and PARI.

## 3. When the specialized construction equals the general one

**Theorem.** Let `I_s ⊂ O0` be a left ideal of norm `N`, and `(u, v)` a `Z[i]`-basis of its lattice (that is,
`Z[i]u ⊕ Z[i]v = I_s`, in numerators). Let `q` be odd, `r ∈ Z[i]` with `N(r) + p ≡ 0 (mod q)`, and
`I_q = ker(ρ|_{O0})`. If `ρ(v)` is a unit of `Z[i]/q`, then

    Z[i]·qv ⊕ Z[i]·(u + γv) = I_s ∩ I_q,    γ ≡ −ρ(u)/ρ(v),

this ideal has index `N²q²` in `O0` (norm `Nq`), and `I_s + I_q = O0` (the ideals are **comaximal**).

The hypothesis `ρ(v)` unit proves comaximality of the *ideals*. It does **not** prove that the integer norms `N` and `q`
are coprime (they could share a factor; see the shared-factor toy sweep in `followup/`). Which conclusion needs which hypothesis:

| Conclusion | Needs |
|---|---|
| lattice equality `⟨qv, u+γv⟩ = I_s ∩ I_q` | `(u,v)` a `Z[i]`-basis of `I_s`, `ρ(v)` a unit mod `q`; nothing about `gcd(N,q)` |
| `I_s + I_q = O0` (comaximal) | the same unit condition (`ρ|_{I_s}` onto) |
| carried norm `T = Nq`, index `N²q²` | the same unit condition (index count in step 4, equivalently CRT for comaximal ideals); not `gcd(N,q) = 1` |
| `gcd(N, q) = 1`, the coprime formula `q·I_s + N·I_q` as a *proved* statement (CRT), and the root `CRT(root_N, root_q)` at modulus `Nq` | **separately**: `q` genuinely prime and `0 < N < q` (so `q ∤ N`). In the signer this is the public Baillie–PSW test on `q` plus Q4 (`N < 2^B ≤ 2^(2e−6) ≤ q`) |

*Proof.* (1) `ρ` is `Z[i]`-linear for left multiplication by Gaussian integers (`J` rotates both components).
(2) `I_q = O0·q + O0·(r + j)`: both generators lie in `ker ρ` (the `O0` generators `1, i, (i+j)/2, (1+k)/2` applied to
`r + j` give numerators `(2r₀, 2r₁, 2, 0)`, `(−2r₁, 2r₀, 0, 2)`, `(−r₁−p, r₀, r₀, 1−r₁)`, ...; the last has `ρ = −(p + N(r)) ≡ 0`, the
others vanish identically). Both contain `q·O0`; in `O0/qO0` the first has at least the `q²` elements
`Z[i]·(r+j)`, and `ker ρ` has exactly `q²` (ρ is onto since `ρ(2,0,0,0) = 2` is a unit), so they are equal. The oracle
verifies this equality as lattices on every row. (3) `w = a·u + b·v ∈ I_q` iff `aρ(u) + bρ(v) = 0`, that is
`b ≡ γa`, which is the `Z[i]`-lattice spanned by `(1,γ)` and `(0,q)`, index `q²` in `Z[i]²`. (4) Hence the index of the
intersection in `I_s` is `q²` as a `Z`-module index, `[O0 : I_s ∩ I_q] = N²q²`, norm `Nq`; and `ρ|_{I_s}` is onto because
`ρ(v)` is a unit, so `I_s + ker ρ = O0`. ∎

Consequences:

* **Comaximality and norm coprimality are different facts.** `ρ(v)` a unit gives `I_s + I_q = O0`, and with it the
  lattice equality and the norm `Nq`. It says nothing about `gcd(N, q)`. Norm coprimality comes from `q` prime and
  `0 < N < q` (here `N < 2^B ≤ 2^(2e−6) ≤ q`), and it is what the Q3 formula `q·I_s + N·I_q` and the CRT root at modulus
  `Nq` are proved under. The toy sweeps in `followup/shared_factor_toy.py` include `q | N` and `q = N` (p = 3 and p = 7,
  `N ∈ {5, 7, 35}`, `q ∈ {5, 7}`): wherever the specialized output exists the ideals are comaximal and the lattice equality and
  the norm `Nq` hold with `gcd(N,q) > 1`; where the ideals are not comaximal (for example `I_s = I_q`) the specialized
  construction refuses, the intersection's norm is not `Nq`, and the Q3 formula is wrong. (The Q3 formula did equal the
  intersection on every comaximal shared-factor toy pair, 612 pairs; I have no proof of that without `gcd = 1`, and the signer does not rely on it.)
* **Primality of `q` is a premise for norm coprimality, not for the lattice equality.** The inverse is verified by
  product-back, which I keep as the check that carries the equality: a composite `q` with a wrong Fermat inverse is marked,
  and if the product-back passes the inverse is right and the theorem applies. But a composite `q > N` could share a factor with `N`,
  so the public Baillie–PSW test is also what gives `gcd(N, q) = 1`.
* **`q ≡ 3 (mod 4)` always.** `s` is odd, so `s² ≡ 1 (mod 8)`, and `8 | 2^(2e−5)`, so `q ≡ 7 (mod 8)`. `Z[i]/q` is a
  field for prime `q`, and `ρ(v)` is a unit iff `ρ(v) ≠ 0` iff `v ∉ I_q`. (The toy sweep with `q ≡ 1 mod 4` shows the other
  case: `ρ(v)` can be a nonzero zero divisor and the check marks the output; 221 of 24,694 attempts, none reachable at
  the five sets.)
* **`v ∉ I_q` is guaranteed by the guard**, for every admitted key at every set: `nrd(v) ≤ 256√p·N < 2^(2e−6) ≤ q`
  and every nonzero element of `I_q` has `nrd` a positive multiple of `q`. Exact check `(256·2^B)²·p < (2^(2e−6))²`
  holds with margins of **398 / 553 / 646 / 898 / 1,226 bits** (P248 / P324 / P376 / P500 / P664; `bounds.py`). So the
  inverse-failure branch is unreachable for honest inputs; it exists as defence and is exercised only on toy primes.
* **The root need only be correct mod `q`.** Representations shifted by multiples of `q`, negated, or with swapped
  coordinates (a root of the same equation) all give the same output (rows `q-root-plus-multiples`,
  `q-root-negated`, `q-root-swap-coords`: ok). The parity lift is not needed for the intersection.
* **No generic-HNF assumption on `I_s` in the mathematics.** The formula works on a Gaussian basis of any left ideal:
  the 218 non-generic `I_sec` of the toy sweep and the `O0·π`, `O0·π̄` (`N ≡ 1 mod 4`) and `O0·ℓ` (norm `ℓ²`) ideals
  at the five real sets all give lattice-equal outputs. What excludes them is **admission**, not the construction:
  `GenericHnf::from_norm_and_root` cannot express them, and the prototype returns `admit=0` for all 15 real-size
  rows (`nongeneric_real.py`). Their reduced bases are also far outside the factor-256 guard (`A ≈ N ≪ √p·N`), so the guard
  alone would refuse them too. The generic shape of `I_q` is automatic: `q ≡ 3 (mod 4)` means every ideal of norm
  `q` has a root.

| Precondition | Where it is checked in the prototype | Oracle evidence |
|---|---|---|
| `(u,v)` is a `Z[i]`-basis of `I_sec`; `I_sec` a left `O0`-ideal of norm `N` | `GuardedIdeal::admit`: determinant `= 4N²`, HNF columns in span with pivot product `4N²`, `O0`-closure on all four integer columns | 280 + 750 verdicts agree, including wrong root, wrong norm, bad parity, `N ≥ 2^B` |
| guard `N4(u) ≤ N4(v)`, `N4(v)² ≤ p(1024N)²`, `A > 0` | `admit` | last-pass / first-fail pairs along three directions at every set, `order-swapped`; all agree |
| `N` odd, `0 < N < 2^B` | `admit` | `N-max-below-2^B` admitted, first prime above `2^B` refused |
| challenge scalar odd, `q` prime, bit length `2e−5` | `intersection` returns `None` | `s-even`, `q-composite`, `s-above-challenge-range` → `none` |
| `p + x² + y² ≡ 0 (mod q)` | `intersection`: sets `ok`, marks all outputs | `q-root-wrong`, `q-root-zero` → all nine marked |
| `ρ(v)` unit mod `q` | `intersection`: product-back check, marks outputs | unreachable at the sets (guard); toy sweeps: every failure is `v ∈ I_q` or, for `q ≡ 1 mod 4`, a zero divisor; never an unexplained failure |
| `gcd(N, q) = 1` (needed for Q3 and CRT, not for the lattice equality or `T = Nq`) | `q` prime (checked above) and `0 < N < q`, from `N < 2^B ≤ q`; not checked at runtime | `bounds.py`; `followup/shared_factor_toy.py` |

## 4. Mismatches

**None.** Specifically no row where the prototype and oracle disagree on admission, on `none` / `marked` / `ok`, on the exact vectors
`U, V, T` (the oracle's independent re-implementation of the formula gives bit-identical vectors, including the
centred-γ convention and sign), on lattice equality with any general route, or on the Gram identities.

Observations, none a defect:

1. The `none` status of `intersection` hides which public check refused. Fine for a public failure.
2. The guard also rejects *small* `N`: at P248 `N` of 119 bits is admitted and 111 bits is refused (reduced bases of
   small-norm ideals are unbalanced, `C ≈ p/A`). Honest keys have `N` near `2^B`; just noting that admission is a
   two-sided range in practice.
3. `rv ∈ I_q` and the zero-divisor case cannot be hit at real sizes, so those branches have toy-prime coverage only.

## 5. Width, relative to the fixed-modulus HNF finding (parent's note)

The fixed-modulus HNF at the intersection modulus `2Nq` needs a Barrett product of 31 / 40 / 46 / 61 / 80 limbs,
above `LIMBS` (26 / 33 / 38 / 49 / 65). **The specialized path never reduces modulo `2Nq`** and builds no
HNF at that modulus. The only modular reduction is by the public odd `q` (Montgomery, `R = 2^(64W)`), whose
products are `< q² < 2^(4e−10)`: 982 / 1,286 / 1,494 / 1,990 / 2,646 bits against the constructor width `64·2W` =
1,408 / 1,792 / 2,048 / 2,688 / 3,456. The outputs are bounded in exact integers (`bounds.py`): `T = Nq ≤ 649 / 838 / 971 / 1,280 / 1,690`
bits, `|U_k|, |V_k| ≤ q√C_max ≤ 638 / 827 / 960 / 1,269 / 1,679` bits, the normalized Gram entries `A', C' < 512√p·q`
(626 / 815 / 948 / 1,257 / 1,667 bits), all below the coordinate width `64W` = 704 / 896 / 1,024 / 1,344 / 1,728 (margins 55 / 58 / 53 / 64 / 38 bits
at the tightest quantity, `T`). The worst-case rows (largest `N`, `v` on the guard boundary in four directions, largest `q`) ran
through the real Rust at every set with **no overflow mark** (observed maxima 648 / 838 / 971 / 1,280 / 1,689 bits). So the specialized
construction avoids the Barrett overrun. This is a size argument plus extreme-input runs; I did not prove a bound on the
intermediates inside `intersection` or `admit`.

## 6. Not covered

* Constant-time behaviour, cycle counts, and the compiled code (G7). Only values are compared.
* `Basis::secret_candidate` (key generation) and the reducer in `examples/stage2c`; I compared the oracle's reduction
  to the committed reducer fixtures only (all 55 reduced bases and normalized forms match, which also covers the half-up and strict-swap rules on
  those inputs).
* Half ties inside the intersection: `γ` is centred modulo an odd `q`, so it has none. A first-step tie in the
  reduction of `(U, V)` needs `2·D(u,v) = (odd)·N4(v)`; for a pre-reduced pair that forces `2|D(u,v)| = N4(v) = N4(u)`, an
  extremal hexagonal-type basis, and I found none at real sizes. Tie handling in the reducer is covered by the committed fixtures
  and the earlier research, not by this oracle.
* Primality of `q` beyond pseudoprimality (the verifier's own premise; it is what gives `gcd(N, q) = 1`).

## 7. Fixtures Rick can commit

`fixtures/<set>.tsv`, 56 rows per set (`fixtures/MANIFEST.txt` has SHA-256 sums). Columns: case, verdict
(`ok`, `reject-admission`, `challenge-refused`, `marked`), input `n xn yn u0..u3 v0..v3 s xq yq` (hex, `-` sign, the
format of `ideals/<set>.txt` extended by the challenge), expected `U0..U3 V0..V3 T`, normalized Gram `A' C' h0' h1'`, and the
reduced `A' C'`. The expected values were produced by the Rust prototype and are bit-identical to the oracle's re-implementation
and lattice-verified. A driver needs `GenericHnf::from_norm_and_root`, `admit`, `intersection`, as in `sio_dump.rs`.
`fixtures/toy_p3.txt` has the `p = 3, T = 5` case (normalized Gram `(10, 10, 4, 9)`, determinant 3; reduced
`u = (4,−1,1,0)`, `v = (1,−2,−2,−1)`, Gram `(2, 2, 0, −1)`) and a hand-checkable intersection (`N = 5`, root `(4,9)`,
`q = 7`, root `(0,5)`; `U = (7,28,0,7)`, `V = (5,−8,4,−1)`, `T = 35`, row-HNF `((1,0,42,53),(0,1,17,42),(0,0,70,0),(0,0,0,70))`,
equal to `GenericHnf(35)` at the CRT root `(14,19)`).

## 8. Reproduce

```
export PATH=/workspace/mamba/envs/sage/bin:$PATH
# build the harness (see rust_harness/README.md), then, in this directory:
python bounds.py                       # exact-integer inequalities, section 3 and 5
python toy.py                          # exhaustive toy primes (results_toy_full.txt)
python medium.py 20000 11              # medium random sweep
python gen_corpus.py 16 1              # structured corpus -> corpus/
for s in p248 p324 p376 p500 p664; do python check.py $s; done
python gen_random.py p500 150 1        # random corpus -> corpus_random/, then check with the corpus-file argument
python check_fixtures.py ROOT BIN      # committed fixtures and ideals corpora
python nongeneric_real.py ; python make_fixtures.py
```
