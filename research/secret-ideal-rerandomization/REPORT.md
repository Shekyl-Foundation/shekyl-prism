# Per-signature re-randomization of the secret ideal

Date: 2026-11-22. Research for Shekyl PRISM. Step-counting and Python
model only; no constant-time or security claim, and not a proposal to
change the signature format.

PRISM is the scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin,
Wouter Castryck, Maria Corte-Real Santos, Riccardo Invernizzi, Antonin
Leroux, Luciano Maino, Frederik Vercauteren, and Benjamin Wesolowski);
`shekyl-prism` implements it. Baseline:
`research/fixed-schedule-reduction/` (exact LLL on `I_sec ∩ I_q` leaks
~2 steps per bit of `N`, closed by a fixed `Z[i]`-Lagrange schedule) and
`research/lagrange-tour-bound/` (certified worst-case tour counts
`T_prov = 301 / 387 / 446 / 587 / 773` for the secret product across
`P248 / P324 / P376 / P500 / P664`).

## Headlines

1. **Re-randomizing `I → J = I · β̄ / N` completely removes the `bits(N)`
   channel from 4-D product reduction, even under exact LLL.** On 150
   keys × 6 messages per set (`pair` panel), baseline exact LLL leaks
   `1.78–2.20` Lovász tests per bit of `N` (`r = 0.316–0.450`,
   `p ≤ 9×10⁻²¹`), and a `dudect` fixed-key-at-`B` vs random-key Welch
   t-test (1,000 vs 1,000 traces) hits **`t = +11.61` to `+19.70`**.
   After re-randomizing `I → J`, the `bits(N)` slope drops to zero within
   95% CI (`−0.21` to `+0.40`, `|r| ≤ 0.073`), between-key `η²` drops to
   the random null (`0.13–0.17` vs null `0.166`, `p = 0.31–0.98`), and
   `odd_gauss` `dudect` `|t|` drops to **`0.09–0.99`**.
2. **You do not need `N'` prime, and Gaussian-then-round beats a box
   sampler.** Requiring `N'` prime (`accept='prime'`) costs `121–301`
   candidates per signature (`K = 5,360–13,350` for `2⁻⁶⁴` fixed-budget
   failure). Requiring only `N' > 1` odd, `bits(N') ≤ B`, and the
   standard cyclic HNF (`accept='odd'`) takes **`2.24–2.28` candidates
   mean** (`0` primality tests, **`K = 75–78`** for `2⁻⁶⁴`) and achieves
   `100%` downstream equivalence and `100%` map-back agreement. A plain
   coefficient box leaks the key's basis skew `shape = bits(nv) − bits(nu)`
   into `bits(N')` (`η² = 0.142–0.176`, `p ≤ 10⁻¹⁹⁵`); shape-normalizing
   cuts it ~6× (`odd_norm`, though a fixed skewed `p500` key with
   `shape = 5` still reached `t = 3.14` in `dudect`); **Gaussian-then-round**
   (`odd_gauss`) removes the shape correlation (`|ρ| ≤ 0.0075`,
   `p = 0.29–0.93`) and cuts `p500` `dudect` to `t = +0.09`.
3. **Certified `Z[i]`-Lagrange already flattens product reduction at
   half the cost, and re-randomization does NOT help `quat_qlapoti`.**
   Using `T_prov = 301 / 387 / 446 / 587 / 773` gives **1 operation
   trace per set** (`t = 0.00`), `100%` shortest-vector recovery (`zi_same
   = 1.0`), and **`0.464–0.475×`** of exact LLL's multiply-and-divide
   count — with zero extra isomorphism bookkeeping. Meanwhile, testing
   re-randomization ahead of `quat_qlapoti` (reusing the C-reference
   `O₀` model from `research/fixed-schedule-norm-equation`) shows that
   `quat_lideal_shortest_equivalent(J · K_J)` lands back on the **exact
   same small norm `n(S)` 100% of the time**, leaving the `S`-dependent
   `quat_qlapoti` leaks untouched (`fixS` max `|t| = 2.11–3.26` vs
   `2.21–3.64` baseline; `ρ(work, v₂(n)) = −0.02` to `−0.14`).
4. **Verdict: optional defence in depth for 4-D reduction, not worth
   adding as primary protection.** On top of the certified fixed
   `Z[i]`-Lagrange schedule (`T_prov`), re-randomizing `I` buys zero
   additional timing flatness on the reduction (already `t = 0`), does
   not fix `quat_qlapoti`, and adds per-signature sampling + HNF/map-back
   bookkeeping (plus a `K = 76–78` fixed-iteration sampler if `tries`
   cannot branch). It is only attractive if an implementation ever runs a
   variable-time 4-D reduction or wants an extra mask against future
   algebraic/fault leaks on `I_sec` — in which case `odd_gauss` (with
   fixed `K = 78` draws) is the right sampler.

---

## 1. How sampling, re-randomization, and bookkeeping work (`rr.py`)

At **key generation** (once per key, non-signing path), the signer
reduces `I` (norm `N`, length `bits(N) ∈ [⌈L/2⌉, B]`, `B = ⌈L/2⌉ + 32`)
with the fixed `Z[i]`-Lagrange schedule sized from `B`, producing a
reduced `Z[i]`-basis `{u, i·u, v, i·v}` with `nrd(u) ≤ nrd(v) ≈ N√p`.

Per **signature**, the signer samples `β = c₀ u + c₁(i u) + c₂ v + c₃(i v) ∈ I`
and sets `N' = nrd(β)/N` and `J = I · β̄ / N`:

- **Box samplers (`plain`, `norm`, `C = 256`):** `c₀, c₁ ∈ [−c_u, c_u]`,
  `c₂, c₃ ∈ [−c_v, c_v]`. `plain` sets `c_u = c_v = C`; `norm` balances
  axis norms via `c_u = ⌊C (nrd(v)/nrd(u))^{1/4}⌋`,
  `c_v = ⌊C (nrd(u)/nrd(v))^{1/4}⌋` (at least `1`).
- **Gaussian-then-round (`gauss`):** draws an isotropic 4-D continuous
  Gaussian `x` under the reduced-norm metric (`E[nrd(x)/N] = 2^{⌈L/2⌉+16}`)
  and Babai-rounds `c = ⌊x B⁻¹⌉` using `B⁻¹` precomputed once at keygen.
  Because `x` is isotropic in `nrd` and far wider than the reduced basis,
  `nrd(β)/N` is asymptotically independent of the basis shape.
- **Acceptance (`prime` vs `odd`):** reject `β = 0`, even `N'`, `N' = 1`,
  and `bits(N') > B`; for `prime`, require `gmpy2.is_prime(N')`; for
  both, require that the HNF of `J` has the standard cyclic shape
  `{N', N'i, α, iα}` (`α = x + yi + j`).

### (e) Isomorphism bookkeeping so verification is unchanged

Because `J = I · g` with `g = β̄ / N`, the left order of `J` is unchanged
(`O_L(J) = O_L(I)`), while its right order is conjugated to
`O_R(J) = g⁻¹ O_R(I) g` (representing the **same** supersingular elliptic
curve `E_pk` up to the explicit isomorphism induced by `β`). Accordingly:

1. **Pull back the challenge ideal:** `K_J = g⁻¹ K_I g` is a left
   `O_R(J)`-ideal of norm `q`, so the composite signing ideal is
   `J · K_J = (I · K_I) · β̄ / N` of norm `N' q`. (Equivalently, in
   `IdealToIsogeny`, `J` connects `O₀` to `O_pk'` and `K_J` pulls back
   along the `β`-isomorphism; in `rr.py`, `rerandomized_product` forms
   `(I ∩ I_q) · β̄ / N` directly.)
2. **Reduce `J · K_J`:** any short element `d' ∈ J · K_J` has
   `nrd(d') / (N' q) = nrd(d) / (N q)` where `d = d' · β / N'`.
3. **Map back:** since `d' ∈ (I · K_I) · β̄ / N` and `β̄ β = N`,
   `d = d' · β / N'` is integral in `I · K_I` (verified `100%` in `pair`:
   `mapped_in = 1.0000` across all 13,500 re-randomized pairs). The
   response ideal `(J · K_J) · d̄' / (N' q)` equals `(I · K_I) · d̄ / (N q)`
   identically. **Neither the signature format nor the verifier changes.**

---

## 2. Sampler cost, acceptance rate, and sampler leakage (`samp` / `sampg`)

1,000 keys × 20 signatures = **20,000 signatures per variant per set**
(`100,000` signatures per variant across `P248–P664`). `η²_null = 0.0500`
(`(1000−1)/(20000−1)`). `K_{2⁻⁶⁴} = ⌈64 ln 2 / −ln(1 − ρ)⌉` is the fixed
number of candidates needed for a constant-time loop to fail with
probability `≤ 2⁻⁶⁴`.

| Set | Variant | `tries` mean / p99 / max | MR tests | HNF checks | `K_{2⁻⁶⁴}` | `bits(N')` mean ± sd (max / `B`) | `tries` `η²` (`p`) | `bits(N')` `η²` (`p`) | `ρ(bits N', shape)` |
|---|---|---:|---:|---:|---:|---|---:|---:|---:|
| `p248` | `prime_plain` | 122.4 / 562 / 1258 | 61.2 | 1.00 | 5,408 | 142.2 ± 1.08 (147 / 158) | 0.0507 (`0.36`) | **0.1761** (`3e-317`) | **+0.224** |
| `p248` | `prime_norm` | 121.3 / 563 / 1027 | 60.8 | 1.00 | 5,360 | 142.0 ± 0.93 (144 / 158) | 0.0474 (`0.88`) | 0.0548 (`0.015`) | −0.030 |
| `p248` | `prime_gauss` | 122.2 / 563 / 1134 | 61.1 | 1.00 | 5,400 | 142.1 ± 1.19 (145 / 158) | 0.0482 (`0.79`) | 0.0534 (`0.057`) | +0.013 |
| `p248` | `odd_plain` | 2.25 / 9 / 19 | 0 | 1.13 | 76 | 142.2 ± 1.08 (147 / 158) | 0.0511 (`0.30`) | **0.1754** (`1e-314`) | **+0.229** |
| `p248` | `odd_norm` | 2.25 / 8 / 18 | 0 | 1.13 | 76 | 142.0 ± 0.91 (144 / 158) | 0.0468 (`0.93`) | 0.0550 (`0.012`) | −0.040 |
| `p248` | `odd_gauss` | 2.28 / 8 / 24 | 0 | 1.13 | **77** | 142.1 ± 1.19 (145 / 158) | 0.0493 (`0.62`) | **0.0507** (`0.35`) | **+0.001** |
| `p324` | `prime_norm` | 152.9 / 722 / 1511 | 76.5 | 1.00 | 6,762 | 179.6 ± 0.92 (182 / 195) | 0.0453 (`0.98`) | 0.0511 (`0.29`) | −0.039 |
| `p324` | `odd_norm` | 2.27 / 9 / 18 | 0 | 1.13 | 77 | 179.6 ± 0.91 (182 / 195) | 0.0505 (`0.40`) | 0.0530 (`0.085`) | −0.033 |
| `p324` | `odd_gauss` | 2.28 / 9 / 19 | 0 | 1.13 | **78** | 179.1 ± 1.19 (182 / 195) | 0.0539 (`0.036`) | **0.0480** (`0.81`) | **−0.008** |
| `p376` | `prime_norm` | 177.5 / 809 / 2023 | 88.9 | 1.00 | 7,850 | 207.8 ± 0.92 (210 / 224) | 0.0467 (`0.93`) | 0.0544 (`0.023`) | −0.023 |
| `p376` | `odd_norm` | 2.26 / 8 / 19 | 0 | 1.13 | 76 | 207.8 ± 0.91 (210 / 224) | 0.0480 (`0.82`) | 0.0523 (`0.15`) | −0.035 |
| `p376` | `odd_gauss` | 2.26 / 8 / 16 | 0 | 1.13 | **76** | 208.1 ± 1.20 (211 / 224) | 0.0511 (`0.30`) | **0.0513** (`0.26`) | **−0.002** |
| `p500` | `prime_norm` | 231.4 / 1055 / 2309 | 115.7 | 1.00 | 10,242 | 269.2 ± 0.92 (271 / 285) | 0.0514 (`0.25`) | 0.0491 (`0.65`) | −0.024 |
| `p500` | `odd_norm` | 2.25 / 8 / 19 | 0 | 1.13 | 76 | 269.2 ± 0.91 (271 / 285) | 0.0483 (`0.77`) | 0.0588 (`5e-5`) | −0.038 |
| `p500` | `odd_gauss` | 2.25 / 8 / 18 | 0 | 1.13 | **76** | 269.1 ± 1.20 (272 / 285) | 0.0472 (`0.89`) | **0.0457** (`0.98`) | **−0.003** |
| `p664` | `prime_norm` | 301.4 / 1386 / 2663 | 150.7 | 1.00 | 13,350 | 350.8 ± 0.91 (353 / 367) | 0.0512 (`0.28`) | 0.0529 (`0.093`) | −0.034 |
| `p664` | `odd_norm` | 2.27 / 8 / 19 | 0 | 1.13 | 77 | 350.8 ± 0.91 (353 / 367) | 0.0519 (`0.18`) | 0.0470 (`0.91`) | −0.032 |
| `p664` | `odd_gauss` | 2.25 / 8 / 16 | 0 | 1.13 | **76** | 351.1 ± 1.18 (354 / 367) | 0.0510 (`0.31`) | **0.0543** (`0.024`) | **+0.001** |

*(Full 30-row table in `results/sampler_table.csv`; `plain` at `p324–p664` repeats `p248` with `η² = 0.142–0.158`, `ρ(bits N', shape) = +0.181` to `+0.211`.)*

Key observations on the sampler:
- **Does `tries` leak the key?** No detectable key effect across 100,000
  draws (`tries` `η² = 0.0453–0.0539` around null `0.0500`, `|ρ(tries, N_bits)| ≤ 0.019`,
  `|ρ(tries, shape)| ≤ 0.013`). However, a variable-time loop still leaks
  the *signature's* `tries` (and thus `N'` parity/HNF rejections), so a
  constant-time signer would pad to `K = 78` draws (for `odd`) or run a
  fixed-count loop.
- **Norm distribution:** `bits(N')` concentrates at `⌈L/2⌉ + 14..16` with
  `sd ≈ 0.91–1.21` bits (`plots/np_bits_hist.png`), staying **11–14 bits
  below `B`** across all 600,000 sampled ideals (`0` overflows).

---

## 3. Downstream 4-D reduction: exact LLL vs certified `Z[i]`-Lagrange

### 3.1 Paired comparison (`pair`, 150 keys × 6 messages = 900/set)

Each `(key, msg)` is reduced on both the baseline product `I ∩ I_q` and
the re-randomized `J · K_J` (with `prime_norm`, `odd_norm`, `odd_gauss`).
Null `key_eta2 = 0.1657`. `Z[i]`-Lagrange was checked both at `T_emp`
(all 900 pairs/set) and at the certified `T_prov = 301/387/446/587/773`
from `research/lagrange-tour-bound` (`verify_tprov.py`, 100 pairs/set).

| Set | Arm | Exact LLL Lovász mean | Slope `steps / bit(N)` ± 95% CI | `r(steps, bits N)` (`p`) | Key `η²` (`p`) | `Z[i]` traces | `Z[i]` short & map-back `zi_same` | Exact LLL `ex_same` | Exact cost vs base |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `p248` | `base` | 1028.2 | **+1.825 ± 0.237** | **+0.450** (`5e-45`) | **0.323** (`2e-14`) | 1 | 1.000 / — | — | 1.000× |
| `p248` | `prime_norm` | 1029.0 | −0.191 ± 0.234 | −0.053 (`0.086`) | 0.153 (`0.76`) | 1 | 1.000 / 1.000 | 0.994 | 1.000× |
| `p248` | `odd_norm` | 1026.2 | −0.022 ± 0.232 | −0.006 (`0.87`) | 0.144 (`0.89`) | 1 | 1.000 / 1.000 | 0.997 | 0.998× |
| `p248` | `odd_gauss` | 1028.4 | −0.147 ± 0.236 | −0.041 (`0.29`) | 0.172 (`0.36`) | 1 | 1.000 / 1.000 | 0.997 | 1.000× |
| `p324` | `base` | 1329.9 | **+1.912 ± 0.269** | **+0.422** (`3e-38`) | **0.324** (`1e-14`) | 1 | 1.000 / — | — | 1.000× |
| `p324` | `odd_norm` | 1333.0 | −0.007 ± 0.266 | −0.002 (`0.71`) | 0.144 (`0.90`) | 1 | 1.000 / 1.000 | 0.992 | 1.003× |
| `p324` | `odd_gauss` | 1330.1 | +0.056 ± 0.274 | +0.013 (`0.57`) | 0.174 (`0.31`) | 1 | 1.000 / 1.000 | 0.994 | 1.000× |
| `p376` | `base` | 1538.0 | **+2.034 ± 0.293** | **+0.414** (`3e-37`) | **0.288** (`6e-10`) | 1 | 1.000 / — | — | 1.000× |
| `p376` | `odd_norm` | 1541.6 | +0.255 ± 0.304 | +0.055 (`0.12`) | 0.147 (`0.85`) | 1 | 1.000 / 1.000 | 0.993 | 1.002× |
| `p376` | `odd_gauss` | 1545.1 | −0.179 ± 0.294 | −0.040 (`0.20`) | 0.156 (`0.70`) | 1 | 1.000 / 1.000 | 0.993 | 1.004× |
| `p500` | `base` | 2039.7 | **+1.781 ± 0.350** | **+0.316** (`9e-21`) | **0.258** (`1e-6`) | 1 | 1.000 / — | — | 1.000× |
| `p500` | `odd_norm` | 2040.9 | −0.083 ± 0.348 | −0.016 (`0.59`) | 0.155 (`0.72`) | 1 | 1.000 / 1.000 | 0.990 | 1.001× |
| `p500` | `odd_gauss` | 2040.3 | +0.400 ± 0.358 | +0.073 (`0.029`) | 0.166 (`0.48`) | 1 | 1.000 / 1.000 | 0.993 | 1.000× |
| `p664` | `base` | 2698.5 | **+2.196 ± 0.381** | **+0.353** (`4e-26`) | **0.283** (`3e-9`) | 1 | 1.000 / — | — | 1.000× |
| `p664` | `odd_norm` | 2704.2 | −0.211 ± 0.391 | −0.035 (`0.25`) | 0.132 (`0.98`) | 1 | 1.000 / 1.000 | 0.998 | 1.002× |
| `p664` | `odd_gauss` | 2701.9 | −0.136 ± 0.374 | −0.024 (`0.43`) | 0.169 (`0.41`) | 1 | 1.000 / 1.000 | 0.997 | 1.001× |

*(Note: `ex_same = 0.990–0.998` reflects exact LLL stopping at `δ = 99/100`
rather than strict `λ₁`; `Z[i]`-Lagrange finds the exact shortest vector
with `zi_same = 1.0000` across all 4,500 pairs and `std_rate = 1.0000`.)*

### 3.2 Certified fixed-schedule `Z[i]`-Lagrange (`T_prov`, `verify_tprov.py`)

Using `T_prov = 301 / 387 / 446 / 587 / 773` (`research/lagrange-tour-bound`)
on both `I ∩ I_q` and re-randomized `J · K_J` (100 pairs/set):

| Set | `T_prov` | `Z[i]` traces | `Z[i]` `mul` + `div` | `Z[i]` `sel` | Exact LLL `mul` + `div` (900-pair mean) | `T_prov` / Exact LLL | Short & `zi_same` (`odd_norm` / `odd_gauss`) |
|---|---:|---:|---:|---:|---:|---:|---|
| `p248` | 301 | 1 | 6,933 + 602 = **7,535** | 3,020 | 15,874.9 | **0.475×** | 1.000 / 1.000 |
| `p324` | 387 | 1 | 8,911 + 774 = **9,685** | 3,880 | 20,523.0 | **0.472×** | 1.000 / 1.000 |
| `p376` | 446 | 1 | 10,268 + 892 = **11,160** | 4,470 | 23,750.2 | **0.470×** | 1.000 / 1.000 |
| `p500` | 587 | 1 | 13,511 + 1,174 = **14,685** | 5,880 | 31,487.6 | **0.466×** | 1.000 / 1.000 |
| `p664` | 773 | 1 | 17,789 + 1,546 = **19,335** | 7,740 | 41,653.6 | **0.464×** | 1.000 / 1.000 |

### 3.3 `dudect` fixed-key (at `B`) vs random-key test (1,000 vs 1,000 traces/set)

| Set | Fixed `N` bits (`B`) | Baseline exact LLL `t` (Lovász / `mul`) | `prime_norm` exact LLL `t` | `odd_norm` exact LLL `t` | `odd_gauss` exact LLL `t` | `odd_gauss` `tries` `t` | `odd_gauss` `bits(N')` `t` | Fixed `Z[i]` `t` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `p248` | 158 | +19.70 / +21.63 | −0.74 / −0.60 | +1.49 / +1.64 | **−0.38 / −0.10** | −1.79 | +1.13 | **0.00** |
| `p324` | 195 | +15.57 / +17.39 | +0.99 / +1.16 | −1.16 / −0.77 | **+0.49 / +0.80** | +0.27 | +0.75 | **0.00** |
| `p376` | 224 | +16.34 / +17.91 | 0.00 / +0.51 | −0.87 / −0.53 | **−0.99 / −1.34** | +2.22 | −0.32 | **0.00** |
| `p500` | 285 | +14.41 / +15.32 | +0.60 / +0.68 | +3.14 / +3.19 | **+0.09 / +0.50** | −0.68 | −1.12 | **0.00** |
| `p664` | 367 | +11.61 / +12.47 | +0.79 / +0.90 | +1.11 / +1.38 | **+0.31 / +0.74** | −0.31 | −0.56 | **0.00** |

The `p500` fixed key happened to have `shape = 5` (skewed reduced basis),
which left a small `t = +3.14` residual under the `odd_norm` box sampler
and vanished (`t = +0.09`) under `odd_gauss`.

---

## 4. Downstream `quat_qlapoti` check (`qlapoti_check.py`)

`research/fixed-schedule-norm-equation` showed that `quat_qlapoti` on
`S = quat_lideal_shortest_equivalent(I_sec · I_q)` already hides
`bits(N)`, yet leaks properties of `S` itself (`v₂(n)` shifts work ~27%,
`fixS` `dudect` `|t|` up to `6.4`). Testing whether re-randomizing
`I_sec → J = I_sec · (−β̄)/N` (which produces the equivalent composite
`L' = J · K_J = L · (−β̄)/N`) alters `S` or its `quat_qlapoti` work
distribution (300 traces per class, `O₀` model from `fne_ref.py`):

| Set | `fixkey` vs `rnd` `t` (base / re-rand) | `fixS` max `\|t\|` (base / re-rand) | `same_n(S)` rate (`fixkey` / `fixS`) | `same_basis(S)` rate (`fixkey` / `fixS`) | `ρ(work, v₂ n)` (base / re-rand) |
|---|---:|---:|---:|---:|---:|
| `p248` | −0.92 / −1.36 | 2.21 / **2.84** | 1.000 / **1.000** | 0.497 / 0.483 | −0.186 / −0.141 |
| `p324` | +0.41 / +1.11 | 2.44 / **3.26** | 1.000 / **1.000** | 0.480 / 0.476 | −0.121 / −0.110 |
| `p376` | +0.20 / −0.45 | 2.36 / **2.11** | 1.000 / **1.000** | 0.470 / 0.463 | −0.248 / −0.024 |
| `p500` | −0.54 / −0.55 | 3.64 / **3.15** | 1.000 / **1.000** | 0.540 / 0.517 | −0.051 / −0.115 |
| `p664` | +0.54 / +0.01 | 2.70 / **2.83** | 1.000 / **1.000** | 0.463 / 0.495 | −0.305 / −0.144 |

Because `L'` and `L` lie in the same left-ideal equivalence class,
`quat_lideal_shortest_equivalent(L')` always lands on the **exact same
minimal norm `n(S)`** (`100%`), merely flipping sign/order ties in 4-D LLL
about half the time. Hence `v₂(n)` and the `S`-dependent `quat_qlapoti`
variance are **unchanged** by secret-ideal re-randomization.

---

## 5. Limits and conclusion

- **Model limits:** Sections 1–3 use the index-4 order `Z⟨1,i,j,k⟩`
  with simulated secret keys (`bits(N) ∈ [⌈L/2⌉, B]`); Section 4 uses
  `fne_ref.py`'s `O₀` model (`PARI` HNF/LLL, Python MT RNG, `gmpy2`
  primality). All costs are operation counts in Python, not CPU cycles,
  and Python big integers are not constant-time.
- **Conclusion:** Per-signature secret-ideal re-randomization works
  mathematically with zero signature-format impact and cleanly masks
  `bits(N)` against a variable-time 4-D LLL (`odd_gauss` reduces
  `dudect` `|t|` from `11.6–19.7` to `0.09–0.99`, needing only `2.25–2.28`
  candidates mean or `K = 76–78` fixed). However, on top of the
  certified fixed-schedule `Z[i]`-Lagrange reduction (`T_prov = 301–773`,
  already `t = 0.00` at `0.464–0.475×` exact LLL), it provides no further
  timing benefit on the reduction and does **not** remove the `S`-level
  leaks of `quat_qlapoti`. We do not recommend adding it to the main
  constant-time signing path, except as optional defence in depth (using
  `odd_gauss` with fixed `K = 78`) if an unhardened 4-D LLL path is ever
  used.
