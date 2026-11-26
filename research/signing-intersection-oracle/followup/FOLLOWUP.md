# Follow-up to the signing-intersection oracle (Rick, t49u and t51u)

Branch `experiment/signing-intersection-oracle-followup`, off `research` at `6f6e1c7`. Prototype: `origin/dev` `470d537`
(unchanged since the first report). The earlier report is [`../REPORT.md`](../REPORT.md); its section 3 is corrected here and there.

## 1. Oracle-only fixtures

`fixtures_oracle/<set>.tsv`, 26 rows per set (19 `ok`, 4 `reject-admission`, 2 `challenge-refused`, 1 `marked`), 130 in all.
Convention and columns are in `fixtures_oracle/README.md`. In short, for every `ok` row the fixture records the carried norm
`T = N·q` and the canonical **column** HNF of `I_sec ∩ I_q` in raw `O0` numerator units: upper triangular, columns are the basis,
positive pivots, entries right of a pivot reduced modulo it (PARI `mathnf` layout, the same as `GenericHnf` columns). It always equals
`[[2T,0,x,2T−y],[0,2T,y,x],[0,0,1,0],[0,0,0,1]]` at the parity-lifted CRT root of norm `T`.

**Separation of provenance.**
* *Oracle-derived* (this directory): inputs from the oracle's generators; expected `T` and HNF from the ideals alone (left closure of
  `{N, root}` and `{q, root_q}`, four independent intersection routes that must agree, plus the generic shape at the CRT root).
  Generated and committed in `5d0cd57` **before** the Rust harness was rebuilt or run.
* *Rust-derived* (`../fixtures/`, merged at `d5acb8e`): expected values taken from the prototype's output, then cross-checked by the oracle.
  Not changed.

**Confirmation** (`confirm_oracle_fixtures.py`, prototype built from `origin/dev` `470d537` with the local harness): **130 / 130 rows agree.**
For the 95 `ok` rows the prototype's carried norm equals the fixture `T`, and the canonical column HNF (PARI `mathnf`) of the span of its
`U, JU, V, JV` equals the fixture matrix entry for entry; the 35 other rows have the same verdict (`admit=0`, `inter=none`, all nine marked).

The `ok` rows per set: 5 prime key norms (3 random, one `≡ 1 mod 4`, the largest prime below `2^B`), 5 composite norms (2 balanced primes,
`3 × large`, `3·5·7 × large`, 3 balanced primes, 4 primes), the prime square, prime cube and `ℓ²·m`, challenge extremes (`q` largest and
smallest, root shifted by multiples of `q`, negated root), and composite-norm keys with `v` on the guard boundary and with unit multiples.
Negatives: composite `q`, even scalar, wrong challenge root, `N` above `2^B`, swapped order, wrong source root, composite key at the first failing shear.

## 2. Composite odd key norms accepted by admission

**Constructed at all five real sets, so no "closest case" is needed.** `composite_sweep.py` builds 90 keys per set (450 in all) with composite odd
`N` within 12 bits of `2^B` and below it, of shapes: two primes (balanced and with a tiny factor), `3·5·7 × large`, three primes, four to seven
small primes times a large one, `ℓ²`, `ℓ³`, `ℓ²·m`, and prime powers of 3, 5 or 7 (for example `3^99` at P248, `5^122` at P500). Each has a generic
root by CRT of roots modulo the prime-power factors, a Gaussian basis from the oracle's reduction (half of them sheared inside the guard), and a
random prime challenge degree. Result: **450 / 450 admitted by the prototype and by the oracle, 450 / 450 intersections `ok`**, every one lattice-equal
to all four general routes, with exact vectors equal to the independent formula, Gram determinant `p`, and reduced `(A', C')` equal to the generic
start's (checked by `check.py` as before). The guard has room: `N4(v)² / (p(1024N)²)` is between `4·10⁻⁶` and `0.055` over the 450 keys, so `N4(v)` is at most 0.24 of its cap.
The earlier structured corpus also had composite, `1 mod 4` and prime-square keys at every set (all admitted and ok).

**What the theorem needs.** Not primality of `N`. It needs, for the key: `(u, v)` is a `Z[i]`-basis of a left `O0`-ideal `I_sec` of norm `N`
(`admit`: determinant `4N²`, HNF columns in the span, `O0`-closure), `N` odd (Hensel inverses), and the guard, which with `q > 2^(2e−6)` gives `ρ(v) ≠ 0`.
For the challenge: `q` odd, a root of `x² + y² + p` modulo `q`, and `ρ(v)` a unit (product-back check). **Primality of `N` is used nowhere in the
construction or in the equality.** What admission does refuse among composites is shape, not compositeness: ideals whose HNF is not the generic
shape (`O0·π`, `O0·ℓ`) cannot be written as `GenericHnf(N, root)` (`../nongeneric_real.py`: all refused). For prime-power norms the generic root must
exist modulo `ℓ^k`, which the sweep constructs by square-root lifting.

## 3. Comaximal versus coprime (Rick, t51u)

Correct as Rick says. **`ρ(v)` a unit mod `q` proves `I_sec + I_q = O0`; it does not prove `gcd(N, q) = 1`.** Which conclusion needs which:

| Conclusion | Needs |
|---|---|
| lattice equality `⟨qv, u+γv⟩ = I_sec ∩ I_q` | Gaussian basis of `I_sec` and `ρ(v)` a unit |
| comaximality `I_sec + I_q = O0` | the same unit condition |
| carried norm `T = N·q` (index `N²q²`) | the same unit condition (index count `q²` in `I_sec`, or CRT for comaximal ideals) |
| `gcd(N,q) = 1`; the Q3 formula `q·I_sec + N·I_q` and the CRT root at modulus `Nq` as proved statements | `q` genuinely prime and `0 < N < q`; in the signer, the public Baillie–PSW test on `q` plus Q4 (`N < 2^B ≤ q`) |

The unit product-back check stays: it is what makes the equality unconditional on how `1/N(ρ(v))` was found. Primality of `q` is a premise for norm
coprimality (a composite `q > N` could share a factor with `N`), not for the lattice equality.

**Evidence** (`shared_factor_toy.py`, exhaustive over all left ideals; `p = 3` and `p = 7`, `N ∈ {5, 7, 35}`, `q ∈ {5, 7}`): on pairs with
`gcd(N,q) > 1` that are comaximal (572 + 40), the intersection has norm `Nq`, and the specialized lattice equals the intersection whenever the
formula applies; on non-comaximal pairs with `gcd > 1` (92 + 15, for example `I_sec = I_q`) the intersection's norm is **not** `Nq`, the Q3 formula is
wrong, and the specialized construction refuses (no output). On comaximal shared-factor pairs the Q3 formula also equalled the intersection in every
case, but I have no proof of that without `gcd = 1` and nothing relies on it. `N = 35`, `q = 5` exercises a *composite* key norm sharing a factor with `q`.
That configuration is impossible at the five sets (`N < 2^B ≤ q`), so it is toy-prime evidence only.

## 4. Files

`gen_oracle_fixtures.py`, `fixtures_oracle/`, `confirm_oracle_fixtures.py` (+ `results_confirm.json`), `composite_sweep.py`
(+ `results_composite_<set>.json`, `comp_<set>.log`), `shared_factor_toy.py` (+ `results_shared_factor_p{3,7}.json`), `common.py`
(imports the earlier `oracle.py`). Run from this directory with the Sage environment; the harness is `../rust_harness/`.
