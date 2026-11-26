# Shortest-equivalent-ideal shortcut: `I·conj(β)/T` without a general HNF

Rick, t47u/t49u/t52u. Branch `experiment/shortest-equivalent-shortcut`, off `research` at `6f6e1c7`. Dev code read at `origin/dev` `470d537`; Rick's handoff at `origin/feat/stage2b-2d-arithmetic` `0b98bd3` (notebook, "Stage 2b/2d order action, intersection and shortest-equivalent handoff"). Research code only; nothing here touches dev.

## Verdict

**Yes.** For every basis `(U,V)` that passes the ideal part of admission, `d = det(U,V)/(2T)` is a Gaussian **unit**, the carried normalized Gram `(A', C', g)` is integral, and

    I·conj(β)/T,  β = U/2  has the Z[i]-basis   U' = (A', 0, 0, 0),   V' = (g_re, g_im, e_re, e_im),     e = det(U,V)/(2T).

No HNF is needed. Multiplying `V'` by the unit `conj(e)` gives the generic-HNF generator `(x', y', 1, 0)` with root `x' + i y' = conj(e)·g`, pivot `2N' = A'`, `N' = A'/2`, and the canonical columns are `(A',0,0,0), (0,A',0,0), (x'',y'',1,0), (A'−y'', x'', 0, 1)` with `x'' = x' mod A'`, `y'' = y' mod A'`. The Gram form of the output is `(A', C', conj(e)·g)`: `A'` and `C'` do not change. Rick's proof sketch is **correct**, with one step it leaves implicit (the 2-adic one, §3).

The result needs only: left `Z[i]`-span, `|det|² = 4T²`, and closure under left multiplication by `j` (a consequence of the `(i+j)/2` check). Parity, odd `T`, primality, the factor-256 guard, the norm bound and the HNF shape are not used for the unit and the Gram. Evenness of `A'` (so `N'` integral) and membership of the output in `O0` use full `O0` closure and parity, which admission also checks.

Notation: numerator `X = (a,b,c,d)` is the quaternion `(z1 + z2·j)/2`, `z1 = a+bi`, `z2 = c+di`, `j z = conj(z) j`, `j² = −p`, `p ≡ 3 (mod 4)`. `M` is the Gaussian matrix with rows `(z1u, z2u)`, `(z1v, z2v)`; `D = det M = z1u z2v − z1v z2u` (this is `cross(u,v)` in `ideal/mod.rs`); `H(w,w') = z1 conj(z1') + p z2 conj(z2')` (the `hermitian` of `geometry.rs`); `A' = N4(U)/2T`, `C' = N4(V)/2T`, `g = H(V,U)/2T = (h0', h1')` (the second `(x,y)` of the normalized Gram).

## 1. Shortcut formulas (exact statement)

For β = εU/2, ε a Gaussian unit (left action; ε=1 is Rick's case), `I' = I·conj(β)/T` is, for any basis `(U,V)` of `I`:

| quantity | formula |
|---|---|
| new norm | `N' = A'/2 = N4(U)/(4T) = nrd(β)/T` |
| first column | `U'_ε = (A'·conj ε, 0, 0, 0)` (`ε = 1`: `(2N',0,0,0)`) |
| second column | `V'_ε = (g·conj ε, e·ε)`, `e = det(U,V)/(2T)` ⇒ for `ε = 1`: `(h0', h1', Re e, Im e)` |
| HNF-shape root | `r = conj(e·ε)·g·conj(ε)`; `ε = 1`: `r = conj(e)·g`; real `e = ±1` gives `r = ±g` |
| canonical column HNF | `(2N',0,0,0), (0,2N',0,0), (r_re mod 2N', r_im mod 2N', 1, 0), (2N' − r_im mod 2N', r_re mod 2N', 0, 1)` |
| second column as β (`β = V/2`) | same formulas with `U ↔ V` |

Why: for `w, w' ∈ I`, `w·conj(w') = (H(w,w') + (z2 z1' − z1 z2')·j)/4`; with `w = V, w' = U` the Gaussian parts are `H(V,U)/4` and `D/4`, so the numerators of `V·conj(U)/T` are `(H(V,U)/2T, D/2T)`, and `U·conj(U)/T = nrd(U)/T = N'` has numerator `2N' = A'`. Right multiplication by `conj(β)/T` is a `Z[i]`-linear bijection `I → I'`, so the images of a basis are a basis.

**Orientation through swaps.** A size-reduction step (`V ← V − m·U − m'·iU`) leaves `D` unchanged. A swap negates it. Multiplying `U` or `V` by a unit `ε` multiplies `D` by `ε`. The generic HNF `U=(2T,0,0,0), V=(x,y,1,0)` has `D = 2T`, so `e = +1`; so a reducer run from the generic basis ends with `e = (−1)^{swaps}` (p=3, T=5: 1 swap ⇒ `e = −1`, checked). For the specialized signing intersection `(qv, u+γv)`, `D = −q·det(u,v)`, hence `e_int = −e_secret`. The unit is therefore `±1` for bases that come from generation or from signing; `±i` appears only for imported bases that are unit multiples of such a basis (Rick's rotations of V by `i^k`: odd `k` gives `e = ±i`).

## 2. Proof that `D/(2T)` is a Gaussian unit (and the Gram is integral)

Facts from admission used (`GuardedIdeal::admit`):
* (F1) `L = Z[i]U + Z[i]V`; `i`-stability is by construction (left mult by `i` is `rotate`).
* (F2) `|D|² = 4T²` with `T > 0` an integer (`abs_squared(det) == index`).
* (F3) `jL ⊆ L`: the `(i+j)/2` check puts `((i+j)/2)·w` in the span for `w ∈ {U, iU, V, iV}`, so `j·w = 2·((i+j)/2)·w − i·w ∈ L`.

**Step 1 (action of `j`).** `j·(z1 + z2 j) = z̄1·j − p·z̄2`, so on numerators `j(z1,z2) = (−p·z̄2, z̄1)`.

**Step 2.** By (F1, F3), `jU = aU + bV`, `jV = cU + dV` with `S = [[a,b],[c,d]]` over `Z[i]`. The rows of `S·M` are the `j`-images, which equal `conj(M)·K`, `K = [[0,1],[−p,0]]`, `det K = p`. Taking determinants:

    det(S) · D  =  p · conj(D)        (exact equality in Z[i]).

**Step 3 (p inert).** `D ≠ 0` by (F2). Norms give `|det S|² = p²`. For `p ≡ 3 (mod 4)` the only Gaussian integers of norm `p²` are `±p, ±pi` (p is inert in `Z[i]`), so `det S = p·η`, `η` a unit, and `conj(D) = η·D`.

**Step 4 (the 2-adic step).** Write `D = α + βi`. `conj(D) = ηD` gives `η = 1`: `D ∈ Z`; `η = −1`: `D ∈ iZ`; `η = i`: `α = −β`, `D = α(1−i)`; `η = −i`: `D = α(1+i)`. (Being associates with the conjugate does **not** by itself make `D/2T` a unit: every `m(1±i)` is associate to its conjugate.) Now use (F2): in the first two cases `m² = 4T²`, so `D = ±2T` or `±2Ti`; in the last two `2α² = 4T²`, i.e. `α² = 2T²`, impossible (`√2` irrational; equivalently `v₂(2α²)` is odd but `v₂(4T²)` is even). The argument never looks at the parity of `T`, so even `T` is covered; 2 ramifies as `(1+i)²`, and the exclusion of `η = ±i` is exactly the statement that `(1+i)` cannot divide `D` to an odd power when `|D|²` is a square. Hence **`D = 2T·ε`, `ε ∈ {±1, ±i}`** (and in fact `η = ε̄/ε = ±1`).

**Step 5 (Gram integrality, no extra input).** Put `Δ = diag(1,p)`; `H = M Δ M*`. From `conj(M) = S M K⁻¹`, `M* = K⁻ᵀ Mᵀ Sᵀ`, `Δ K⁻ᵀ = Ω = [[0,1],[−1,0]]` and `M Ω Mᵀ = D·Ω`:

    H = D · Ω · Sᵀ,   i.e.  H(U,U) = D·b,  H(U,V) = D·d,  H(V,U) = −D·a,  H(V,V) = −D·c.

So `Gram/(2T) = ε·Ω·Sᵀ` is an integral Gaussian matrix: `A'`, `C'` and `g = −ε·a` are integers/Gaussian integers, with no assumption beyond (F1–F3). (Checked on every row below.)

**Step 6 (evenness and the output ideal; these use parity and full `O0` closure).** A left `O0`-ideal `I` of index `T²` in `O0` satisfies `I·conj(I) = T·O0` (`O0` is maximal, so `I` is locally principal, `I_ℓ = O_ℓ α_ℓ`, `I Ī = nrd(α_ℓ) O_ℓ` locally and the index is `∏ ℓ^{2 v_ℓ(nrd α_ℓ)} = T²`). Hence `w·conj(w')/T ∈ O0` for `w, w' ∈ I`: the numerators `(H(w,w')/2T, D(w,w')/2T)` are integral (again, independently of Step 5), `nrd(U)/T ∈ Z` so **`A'` and `C'` are even** and `N' = A'/2 ∈ Z`, and `I' = I·conj(β)/T ⊆ O0` is a left `O0`-ideal of norm `N' = nrd(β)/T`. Parity of `O0` gives for the output `x'` even, `y'` odd, as the generic HNF requires.

**Which checks are therefore needed.** Unit + Gram integrality: (F1–F3), i.e. span, index `4T²`, `(i+j)/2` closure. Evenness / `N'` integral / output in `O0`: add the parity check (`L ⊂ O0`). Not needed: `T` odd, `T` prime, the guard, `T < 2^B`, HNF shape, reducedness. **Necessity:** the lattice `O1 = Z[1,i,j,k] ⊂ O0` (basis `U=(2,0,0,0)`, `V=(0,0,2,0)`, `T=2`) is `j`- and `i`-closed, inside `O0`, index `T²`; `D/2T = 1` is a unit and the Gram is integral, but `A' = 1` is odd (not `(i+j)/2`-closed): so `j`-closure alone does not give evenness. Without closure, `D/2T` need not even be integral: of 139 random `O0`-parity lattices with `|D|² = 4T²`, 47 have a non-integral quotient (`negative_controls.py`).

## 3. Review of Rick's proof sketch ("`d | p·conj(d)` from left multiplication by `j`; p inert ⇒ associates")

* **The `j`-action claim holds for every admitted basis, imported ones included.** It uses (F1) and (F3), which `admit` enforces on `U, V, iU, iV` (the `Span::contains` of the half-product), and (F2). It does not use parity, oddness or the guard. For the imported basis the membership is exactly what the `twice`/`contains` loop checks. One precision: the relation is the exact equation `det(S)·D = p·conj(D)` (so `|det S| = p` follows from `D ≠ 0`), not just a divisibility.
* **"p inert ⇒ associates" is right** (Step 3: `|det S|² = p²` ⇒ `det S = pη`).
* **Gap (small): associates + `|D| = 2T` is not enough alone.** `D = m(1±i)` is associate to its conjugate. One needs the 2-adic exclusion of Step 4 (`2α² = 4T²` has no integer solution), for any `T`, even or odd. With it the conclusion `D/(2T) ∈ {±1, ±i}` is complete.
* Not covered by the sketch (the claim the shortcut also needs): integrality of `A', g` for imported bases and evenness of `A'` — Step 5 gives the first from the same matrix `S`; Step 6 (parity + `O0`-closure) the second.
* The `1±i` cases are impossible, so `η = ±1` always: conj(D) = ±D. Hence `D` is real or purely imaginary.

## 4. Is the output directly a valid reduced/admissible basis?

* **Valid Z[i]-basis of the right ideal: yes, and in HNF-generator form** (after the unit `conj(e)`). No further step for correctness.
* **Reduced: up to half-ties.** The Gram is `(A', C', conj(e)·g)`: `A' ≤ C'` unchanged, `|g|` unchanged, and rotating `g` by a unit keeps the box `|Re|,|Im| ≤ A'/2`. But `floor(h/A + 1/2)` rounds `+A/2` up and `−A/2` down, so an input tie at `−A'/2` can become `+A'/2` (e.g. `p=3, T=5`: reduced `g = (0,−1)`, `A' = 2`; `e = −1`, output `g = (0, 1)`, one more size-reduction step would act). At real sizes none of the 310 reduced inputs hit a tie; the toy exhaustive run has 648 such tie-only cases and no other non-reduced output.
* **Admissible (`GuardedIdeal::admit`): the guard transfers exactly** (`C'` is unchanged, `c ≥ a` ⟺ `C' ≥ A'`, and `c² ≤ p·cap²` ⟺ `C' ≤ 2⁹√p`, independent of T). `N' ≤ √(p/2)` for a size-reduced input, far below `2^B`. **But `admit` requires `T` odd** (`norm.is_odd()`; `Span::contains` inverts `T²` modulo `2^{64W}`): about 40 % of outputs have `N' = nrd(U)/T` even (22 of 55 on Rick's corpus; 3,291 of 7,300 orientation outputs here), and those are not admissible there. That is a limit of `admit`, not of the shortcut: the formulas, the HNF and the Gram are valid for even `N'` (A' ≡ 0 mod 4), which the toy runs cover with every even norm up to 12. A consumer of the shortest equivalent ideal that needs odd norm must choose a different `β` (any `β ∈ I` works; the norm is `nrd(β)/T`) or take the Gram directly.

## 5. Evidence (exact integers, Sage; independent of the Rust prototype)

Reference: for each basis and orientation, the Z-lattice of `{ w·conj(β)/T : w ∈ {U, iU, V, iV} }` by direct quaternion multiplication (`oracle.qmul`), exact-division checked, canonical row HNF (Sage), versus the lattice of the shortcut basis, versus the canonical generic-HNF columns built from `(N', r)`; plus left-`O0`-ideal and norm-`N'` checks, Gram invariance `(x'²+y'²+p)/(2N') = C'` and `|g|² = A'C' − p`, `S`/`det S = pη`/`H = DΩSᵀ` (`j_identities`), the guard transfer, and output admission by an independent re-implementation of the checks. Orientations: `β = εU/2` for the 4 units plus `β = V/2` (5 per basis).

| corpus | bases | orientation checks | result |
|---|---:|---:|---|
| Rick's 55 dev `stage2c/vectors` rows × returned (reduced) columns × 4 rotations of V (`dev_tally.py`) | 55 | 220 | all unit; 110 real (`±1`), 110 imaginary; **22 even `N'`** (exactly Rick's counts); lattice equal 220/220 |
| real sizes, five sets: dev vectors (11 per set), dev ideals (6), signing-oracle fixtures (39 `ok` rows) each as given, reduced, 3 random unimodular twists (up to 40-bit multipliers) and 3 twists of the reduced basis; `run_real.py` | 1,460 | 7,300 | 0 failures; `A'`, `C'` even 1,460/1,460; `j`-identities 1,460/1,460; 3,291 even-`N'` outputs |
| toy primes 3, 7, 11, 19, 23, 31, 43: **every** left `O0`-ideal of norm ≤ 12 (≤ 10 for the last three), `O0·n + O0·α` over all `α mod n`, including even, composite and `p | n` norms; 10 bases each (Gaussian Euclid, reduced, twists); `toy_exhaustive.py` | 15,420 | 77,100 | 1,542 ideals, 0 failures; 35,456 even outputs; `j`-identities 15,420/15,420 |
| negative controls (`negative_controls.py`) | – | – | closure needed for integrality (47/139 non-integral); `j`-closed `O1` has odd `A'`; the `(1±i)` family never has `|D|² = 4T²` |
| `p=3, T=5` | 2 | 10 | generic basis `e = +1`, root `(4,9)`, canonical columns `(10,0,0,0),(0,10,0,0),(4,9,1,0),(1,4,0,1)`; reduced basis `e = −1` after one swap, `N' = 1`, `O0` itself, root `(0,1)` (tie case) |

The `twisted` bases are not size-reduced and most fail the guard; they are there to test (F1–F3) on imported bases, not to be admissible.

## 6. Cross-check of `fixed-modulus-hnf` against an unbounded oracle in dev (brief)

`origin/dev` (`470d537`) has no unbounded lattice/HNF oracle for the ideal code; its only bignum oracle is `prism-int/tests/oracle.rs` (`num-bigint`, integer ops). The unbounded column-HNF oracle Rick describes is `shekyl-prism/tests/signer_layers/ideal_lattice.rs` on `origin/feat/stage2b-2d-arithmetic` (`6f693a0`, `0b98bd3`): a `BigInt` column HNF of a generator list, used to compare the signing intersection with the general construction. It is an exact HNF, not a width- or step-bounded fixed-modulus algorithm, so it checks lattices and canonical form but cannot check the `fixed-modulus-hnf` operation counts or limb widths. Those remain model-level (Sage `hermite_form` and PARI `mathnf` agreed on 1,745 adversarial runs plus the decode shape). That oracle is a suitable independent comparison for a native implementation of the `2N` schedule; nothing found here conflicts with the bounds. (The shortcut above removes the need for the general HNF in this path.)

## 7. Limits

Exact-integer model only; no constant-time claim, no timing or width claim beyond: `D/2T` fits wherever `D` does, `A', C' ≤ 2^9√p` for an admitted guarded basis; the Gaussian determinant, `H`, and one division by `2T` are exact integers and an implementation should still check divisibility (a free product-back check) rather than rely on the proof, until the proof is accepted. The ideal-theoretic Step 6 uses the standard local structure of ideals of a maximal order; Steps 1–5 are elementary and checked symbolically/numerically only (`j`-identities hold on all 16,880 bases of the real-size and toy runs). The proof was not checked in Lean/Coq.

## Files

`sq.py` (formulas, direct reference, admission predicate, `j`-identities), `check_lib.py`, `run_real.py`, `toy_exhaustive.py`, `dev_tally.py`, `negative_controls.py`, `oracle.py` (unchanged copy of `signing-intersection-oracle/oracle.py`), `results/`. Dev vectors read from `origin/dev` `470d537` (`shekyl-prism/examples/stage2c/vectors|ideals`, copied to `/tmp/dev_fx` for the run; not committed). Run: `PATH=/workspace/mamba/envs/sage/bin:$PATH python run_real.py p500` etc.
