# Stage 2c design notes: input admission and the prism-int width mapping (SHE-25)

Branch `experiment/stage2c-design-notes`, off `dev` at `904d876` (G7 landed through PR #44). **These are notes and proposals for Rick to decide on, not adopted spec.** Nothing here changes `docs/SIGNER_SPEC.md`, `prism-int` or any crate. No production reducer is built. The scripts are small stdlib-only Python checks.

Sources read:

- `dev` `904d876`: SIGNER_SPEC sections 0, 2.1.2 to 2.1.4, and `prism-int` (`SecretInt`, `SecretBit`, the sticky overflow mark, the three exits, `surface.txt`).
- PR #45 head `84a798a` (`signer/stage-2c-reduction`; `abed5c1` plus the units footnote): SIGNER_SPEC 2.3 and `scripts/stage2c_oracle.py`.
- Rick's handoff commits on `experiment/normalized-schedule-handoff`: `df92507` (tail-only first steps) and `a80245c` (native prototype handoff).
- Rick's `a60fd06` (`experiment/gaussian-reduction-alternatives`).
- On `research`: `normalized-gram`, `q5-width-proof`, `clamped-quotient-progress`, `tighter-progress-certificate`, `ct-barrier-policy` and `ct-feasibility-gate`.

## Summary of recommendations

| Item | Recommendation |
|---|---|
| 1. Admission and failure | **Option D.** A typed `AdmittedBasis` whose constructors only trusted producers can call. Key generation and decode check every invariant, combine the results into one `SecretBit` and declassify it once, which takes two new rows in the exit table. Signing does not declassify. The constructor's checks still run, and the result travels as a status bit. The signer folds that bit into the overflow mark (new `mark_if`) just before the signature's existing exits. Signing gets no new exit and no mid-signing branch, and a failure looks exactly like an overflow, whose error is already treated as **U**. |
| 2. Widths and scratch | Use per-purpose widths at each set: **Gram W = coordinates C = 11/14/16/21/27 limbs** and **constructor 2W = 22/28/32/42/54**. Both are proven, and both fit under `LIMBS` 26/33/38/49/65. This needs a ruling against the one-width rule of 2.1.2. Every honest intermediate is shown below its width, including the discarded exp64 path, so the mark is never set on admitted inputs. The long-lived state goes in a caller-owned `ReduceState`, and temporaries stay on the stack. The estimated peak at P664 is about 9.8 KB (17 KB at the single width). Section 2.7 lists the missing `prism-int` primitives with signatures: 13 on `SecretInt` and 3 on `SecretBit`. |
| 3. Consumer test | Convert the carried Gram exactly: `F00=F11=A'/2`, `F22=F33=C'/2`, `F20=h0'`, `F21=h1'`, `F30=−h1'`, `F31=h0'`. The test checks the identity, the terminal state read off `F`, a fresh-versus-carried comparison and the lattice. The p=3, T=5 fixture passes at the full schedule and **fails at one step**, while a shortest-vector check passes at one step. Worked by hand in section 3.3 and checked by `check_fixture.py`. |
| 4. Schedule as data | `steps(set, class, table)` is public and constant. `class` is carried by `AdmittedBasis`, and `table` is `Pr45Reviewed` (the default) or `JointCandidate`. One loop takes `steps: u32`. The perf protocol runs the same binary at 609/546 steps and also 594/533, the guarded pre-reduced row that the native signer will actually use. |

---

## 1. Input admission and failure handling

### 1.1 What must hold before the reducer runs

The widths (section 2) and the schedule row (section 4) are proven only for inputs that satisfy all of the following. The table also says who establishes each one.

| Invariant | Generic-HNF classes (keygen norm D, secret-ideal pre-reduction, challenge alone, generic signing) | Guarded pre-reduced signing (`U=q·vₛ`, `V=uₛ+γ·vₛ`) |
|---|---|---|
| Norm `T` carried, never recovered (Q2) | 2b's HNF type carries it | 2d computes `T = N·q` from the admitted key and the public `q` |
| Membership and left-O0 stability | By construction: the HNF of the ideal, with the fixed modulus of Q1 | From key admission (`uₛ, vₛ` span `I_sec`) and 2d's intersection argument. **Integrality plus det = p is not shown to imply O0 stability.** Decode must check it separately (1.6). |
| Span: the four columns are a basis | By construction (HNF) | Runtime check `A'C' − h0'² − h1'² = p`. Given membership, this is equivalent to spanning (normalized-gram section 5). |
| Shape `U=(2T,0,0,0)`, `V=(x,y,1,0)`, x even, y odd, in `[0,2T)` | Runtime check, folded into the status bit (cheap mask tests) | Not used |
| `T` odd (Hensel) | Runtime bit | Runtime bit |
| Integrality: diagonal / 4T, projections / 2T | Runtime: `4 | n`, then Hensel and product-back `Q·T = n/4` | Runtime: four Hensel divisions with product-back |
| Class bound `M0` (the widths depend on it) | By construction: `T < Tmax` (Q4 at admission and public `q`), `x, y < 2T` | By construction from the guard (checked at keygen and decode) and `|γ| ≤ q` (2d). Optional runtime `A', C' ≤ M0`: two `ct_lt`, recommended as defence. |
| Factor-256 guard `0<nrd(uₛ)≤nrd(vₛ)≤256√p·N` | n/a | **Key generation and decode both check it** and carry it in `AdmittedKey`'s type. Signing does not recheck. |

### 1.2 Where a failure can arise

| Site | How often | Input | What a failure means |
|---|---|---|---|
| Key generation, candidate test (Q4 norm, guard, HNF shape) | Once per draw | Fresh secret draw | Expected: reject and draw again (Q4 already searches) |
| Key decode | Once per import | Bytes the holder supplies | Expected for a bad key: refuse the key |
| 2c constructor in signing | Every signature | Derived from an admitted key and the public challenge | **Unreachable by proof** if 2b/2d and admission are correct; a failure is a bug |
| 2c constructor in keygen's own reductions | Once per key | Derived from the candidate | Generic-HNF shape failure is reachable with probability about 1/ℓ (SIGNER_SPEC 2.1.2 row 4). Fold it into the candidate test. |
| `prism-int` overflow inside the reducer | Every step | Admitted | Unreachable by proof (section 2.5) |

### 1.3 Options

The declassification review rule (SIGNER_SPEC 0 and 2.1.3) requires a row for every call site of an exit. Each row gives a treatment (F, I, B or E) and the treatment of its overflow error (U).

**Option A: a `CtOption`-like result, declassified at the constructor.**

```rust
pub(crate) fn admit_generic(h: &GenericHnf<C>, set: &SetConsts) -> (AdmittedBasis<C, W>, SecretBit);
// caller: if !ok.declassify()? { return Err(SignError::Invariant) }
```

- *Secret branch:* one branch on a key- and challenge-derived bit, in the middle of signing, at every signature.
- *Exit:* `SecretBit::declassify`, one row per call site (sign, keygen-D, Isec, challenge). Treatment I is justified only by the same lemma that makes the bit constant. Overflow: U.
- *Cost of being wrong:* if the lemma fails, an attacker who picks messages gets an early-abort oracle at a known point. The abort also changes the instruction count, which C avoids.
- *API:* explicit, and the easiest to test.

**Option B: invariant-carrying types from trusted sources only, with no runtime check.**

```rust
pub(crate) fn admit_generic(h: &GenericHnf<C>) -> AdmittedBasis<C, W>;  // debug_assert only
```

- *Secret branch:* none. *Exit:* none.
- *But:* the Hensel quotient means nothing without its product-back check. A bug in 2b/2d then gives a wrong Gram silently, and the reducer runs on it with no signal. The check costs about 0.2% of the reduction (normalized-gram section 2), so skipping it saves nothing. Rejected on its own; its type discipline is kept in D.

**Option C: fold the failure into the sticky overflow mark.**

```rust
impl<const N: usize> SecretInt<N> { pub fn mark_if(&mut self, bit: &SecretBit); }  // monotone
```

- *Secret branch:* none. The same instructions run whether the check passes or not, which is the design goal of `prism-int` (2.1.3).
- *Exit:* none new. The failure appears as `SecretError::Overflowed` at whatever exit later reads a marked value. That error column must already be **U**.
- *Risk:* it relies on the mark propagating to an exit. `select` passes on only the mark of the value it selects (2.1.3), so a mark set early on one Gram entry could in principle be dropped by a selection whose bit is not derived from that entry. It also widens the meaning of "overflowed" to "overflowed or invalidated".

**Option D (recommended): B's types and C's mark, with the status bit carried explicitly.**

- **Admission boundaries (keygen candidate, key decode).** Combine every invariant into one `SecretBit` with the new `SecretBit::and`, and declassify it once.
  - Keygen: row "candidate accepted". Treatment **I** by rejection sampling: a rejected draw is discarded, and with fresh draws the accepted key is independent of how many were rejected. Q4's norm test and the guard share this one row. Overflow: **U**, sizing the checks from `widths.py`.
  - Decode: row "key valid". Treatment **I**: the bit is constant true on every key the generator can emit, and false only says the supplied bytes are not a key. Overflow: **U**, sizing the check width from the encoding (section 2.2, row 30).
- **Inside an operation (2c constructors, called from signing and from keygen's reductions).** The constructor computes the same kind of bit and does **not** declassify it. `AdmittedBasis` carries it, and `Reduced` carries it on, without dropping it. The signer ANDs the status bits of all stages and calls `mark_if(!ok)` on the values the signature's existing exits will read, just before those exits. That needs no mark-flow argument through stages 3 and 4, and no new row in the exit table. The signature exits' overflow rows are already U. They become "U: overflow unreachable by 2.5, invariant failure unreachable by the admission lemmas of 1.1".
- **API shape:**

```rust
pub enum InputClass { SignGeneric, SignPreReducedGuard256, KeygenNormD, SecretIdealPreReduction, ChallengeAlone } // public

pub struct AdmittedBasis<const C: usize, const W: usize> {
    class: InputClass,                 // public: selects M0 (width proof) and the schedule row
    u: [SecretInt<C>; 4], v: [SecretInt<C>; 4],
    gram: NormGram<W>,                 // (A', C', h0', h1'), numerator entries / 2T
    ok: SecretBit,                     // AND of the constructor's checks; never declassified in 2c
}
impl<const C: usize, const W: usize> AdmittedBasis<C, W> {
    pub(crate) fn from_generic_hnf<const W2: usize>(h: &GenericHnf<C>, class: GenericClass, set: &SetConsts) -> Self;
    pub(crate) fn from_guarded_intersection<const W2: usize>(
        key: &AdmittedKey<C>, q: &Int<C>, gamma: &[SecretInt<C>; 2], set: &SetConsts) -> Self;
}
pub(crate) fn reduce<const C: usize, const W: usize>(
    b: AdmittedBasis<C, W>, table: ScheduleTable, st: &mut ReduceState<C, W>) -> Reduced<C, W>;
pub struct Reduced<const C: usize, const W: usize> { /* u, v, gram, t, ok */ }
impl Reduced { pub fn form(&self) -> ClassForm<W>; pub fn status(&self) -> &SecretBit; }
```

  `GenericHnf` is produced only by 2b's Q1 routine, and `AdmittedKey` only by keygen and decode. Constructors are `pub(crate)`, so no public path builds an `AdmittedBasis` from raw integers. Tests read the status bit through a `#[cfg(test)]` accessor, which adds no row to the exit table.

- **What a signer does on `Overflowed` at its exit** (stage 5's decision, not 2c's): return an error and keep no automatic retry counter. 2.1.2's "abandon and draw again" applies only if a proof is wrong, and a retry count that depends on a secret is a channel.

### 1.4 Draft rows for the 2.1.3 table (under D)

| Stage | Call site | Exit | What leaves | Treatment | Overflow error |
|---|---|---|---|---|---|
| 5 | keygen, candidate test | `SecretBit::declassify` | accept/reject of one draw (Q4, guard 256, HNF shape of the norm-D and secret-ideal bases) | I (rejection sampling with fresh draws) | U (widths 2.2) |
| 5 | key decode | `SecretBit::declassify` | valid/invalid key | I (constant on generated keys) | U (encoding-sized widths) |
| 2c | none | — | — | — | the signature exits' U now also covers `mark_if` |

### 1.5 Admission lemmas the U treatment rests on

For each runtime check, a reason it passes on every admitted input:

1. **Generic HNF shape and parity:** 2b's output type has this shape by construction. In keygen the shape bit is part of the candidate test.
2. **`4 | x²+y²+p` and `T | (x²+y²+p)/4`:** `(x, y, 1, 0)` lies in an ideal of norm T, so its reduced norm `(x²+y²+p)/4` is a multiple of T.
3. **Diagonal / 4T, projections / 2T (general constructor):** every element norm of an integral norm-T ideal is a multiple of T. Polarizing over `U`, `JU`, `V` and `U+V` gives the projections (SIGNER_SPEC 2.3).
4. **det = p:** given membership, the Gaussian pair spans the ideal (`|det B| = 4T²`, PR #45 2.3).
5. **`A', C' ≤ M0`:** the class bounds of `widths.py`, from Q4, the public `q`, the guard and `|γ| ≤ q`.
6. **`T` odd:** every admitted class has odd T: `N` and `q` are odd primes and `D` is prime (normalized-gram section 6).

### 1.6 What the 2c constructor cannot check, for 2b, 2d and decode to own

- **O0 stability (membership) of a decoded `(uₛ, vₛ)`.** A Z[i]-lattice with integral normalized Gram and det = p is not shown to be a left O0-ideal. Decode needs its own check, for example the HNF of `{uₛ, Juₛ, vₛ, Jvₛ}` together with O0's generators applied, under Q1's fixed modulus `2N`, compared with the stored ideal. Owner: 2b/2d. **Open.**
- **HNF shape in signing.** If the native signer signs with the generic-HNF class rather than the guarded intersection, a non-generic `I_sec ∩ I_q` becomes a reachable, secret-dependent failure with probability about 1/ℓ. It would then need an entry in the failure budget instead of U. Rick's handoff (`a80245c`) uses the intersection basis with the general constructor, which avoids this.
- **Reading key bytes.** `from_le_bytes` sets the mark on bytes that do not fit. At decode that is just another rejection, and it surfaces at the decode row's declassify as `Overflowed`. Map it to "invalid key".

---

## 2. The prism-int width mapping

### 2.1 Widths

Every limb count comes from `widths.py`, which restates the bounds of `normalized-gram/bounds_ng.py` (W1 to W4) and adds W5 (2.4). `prism-int` holds sign and magnitude separately, so a magnitude below 2^(64N) fits in N limbs and no limb is spent on the sign.

| Set | `M0` bits (generic signing, binding) | `9M0` bits | Gram `W` | Coordinate bits | Coordinates `C` | Constructor numerator bits | `Q·T` bits | Constructor `2W` | Decode guard limbs | `LIMBS` (2.1.2) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| P248 | 652 | 655 | **11** | 653 | **11** | 1,303 | 1,354 | **22** | 10 | 26 |
| P324 | 841 | 844 | **14** | 842 | **14** | 1,681 | 1,735 | **28** | 12 | 33 |
| P376 | 974 | 977 | **16** | 975 | **16** | 1,947 | 1,996 | **32** | 14 | 38 |
| P500 | 1,283 | 1,286 | **21** | 1,284 | **21** | 2,565 | 2,625 | **42** | 18 | 49 |
| P664 | 1,693 | 1,696 | **27** | 1,694 | **27** | 3,385 | 3,419 | **54** | 23 | 65 |

- Spare bits above the binding bound: Gram 49/52/47/58/32, coordinates 51/54/49/60/34.
- Generic signing binds at every set. The guarded pre-reduced class binds the coordinates at P664 (1,681 bits), still under 27 limbs. The per-class rows are in `results/widths.txt`.
- **Ruling needed (2.1.2, "one width per set").** Everything fits the single width `LIMBS`, since 2W ≤ LIMBS at every set. The single width, though, costs `LIMBS²` limb products per multiplication against `W²`: 2,401 against 441 at P500, which is 5.4 times more. That would undo the normalized-Gram speedup. 2.1.2 already says that a later stage needing another width changes a constant and says why. This is that case. Proposal: three named constants per set, `GRAM = COORD = W` and `CTOR = 2W`, in the signer crate. `prism-int` stays generic over N.

### 2.2 Operation map

"New" means a primitive from 2.7. "Honest bound" is the reason the mark cannot be set on an admitted input.

| # | Research operation (`a60fd06`, `normalized-gram`) | Widths | prism-int today | Needed | Honest bound, so no mark |
|---|---|---|---|---|---|
| 1 | Bring 2b's `T, x, y` (at `LIMBS`) to `C` | LIMBS → C | — | new `resize` (marks on lost bits) | `T < 2^Tbits`, `x, y < 2T`, both under 64C |
| 2 | `n = x² + y² + p` | 2W | `mul`, `add` after `resize` to 2W | (optional `mul_widen`) | `n < 8Tmax² + p`, 1,303 to 3,385 bits, under 128W |
| 3 | `T` odd, `x` even, `y` odd, `4 | n` | bits | none | new `is_odd`, `low_bits_zero(k)` | — |
| 4 | `n >> 2` | 2W | `shr(2)` (magnitude), exact once (3) holds | — | — |
| 5 | `T⁻¹ mod 2^(64W)`: 5-bit seed `3T xor 2`, Newton steps (public count) | W | `mul` would **mark** on wrap | new `mul_low`, `truncate` (modular, never mark) | modular by definition |
| 6 | Hensel `Q = (n/4 mod 2^(64W))·T⁻¹ mod 2^(64W)` | 2W → W | `resize` would mark | new `truncate::<W>`, `mul_low` | modular |
| 7 | Product back `Q·T == n/4` | W×W at 2W | `mul` at 2W, `ct_eq` | — | `Q < 2^(64W)` and `T < 2^(64W)`, so the product is below 2^(128W) and fits whether or not the check passes |
| 8 | `A' = 2T`, `C' = 2Q` | W | `shl(1)` | — | `≤ M0` |
| 9 | General constructor: `N4(U)`, `N4(V)`, `D(V,U)`, `D(V,JU)` | C → 2W | `mul`, `add` after `resize` | — | `≤ 2Tmax·M0`, up to 3,358 bits at P664 |
| 10 | Projection / 2T: low bit, `shr(1)`, Hensel on the magnitude, sign put back | 2W → W | `shr`, `select(x, x.neg())` | new `abs`, `is_negative` | sign and magnitude make this exact; no floor shift is needed |
| 11 | `A'C' − h0'² − h1'² == p` | 2W | `mul`, `sub`, `ct_eq` | — | `≤ M0²` |
| 12 | Optional `A', C' ≤ M0` | W | `ct_lt` | — | — |
| 13 | Combine checks into `ok` | bit | **none: `SecretBit` has only `declassify`** | new `SecretBit::and`, `or`, `not` | — |
| 14 | Initial strict ordering: `C' < A'`, swap `U, V` and `A', C'`, negate `h1'` | C, W | `ct_lt`, `swap`, `select`, `neg` | — | — |
| 15 | `bl(A)`, `bl(|h0|)`, `bl(|h1|)` | W → 1 | none | new `bit_length` returning `SecretInt<1>` | `≤ 64W` |
| 16 | `k = max(0, bl(A) − 64)`, `e`, `big = e > 60`, `s = big ? e − 60 : 0` | 1 | `add`, `sub`, `ct_lt`, `select` on `SecretInt<1>` | — | counts below 2^11 |
| 17 | `n = A >> k`, its low word | W → 1 | public-count `shr` only | new `shr_secret`, then `resize::<1>` | `bl(A >> k) ≤ 64` (W5a) |
| 18 | `num_j = |h_j| >> (k+s)` as two words | W → 2 | — | `abs`, `shr_secret`, `resize::<2>` | `< 2^124` on both paths (W5b) |
| 19 | `qtop_j = num_j / n`, fixed 64-iteration 128-by-64 division | 2, 1 → 1 | none | new `SecretInt<2>::div_word` (marks if `hi ≥ n` or `n = 0`) | `hi < n`, `qtop < 2^61` (W5c) |
| 20 | `mw_j = sign(h_j)·qtop_j` | 1 | `select`, `neg` | — | — |
| 21 | `X = 2h + A − 2·mw·A` | W×1 | — | new `mul_word` | `|X|, |X−2A| < 9M0` on **both** paths (W2, W5e) |
| 22 | Signed half-up correction: `lo = X<0`, `hi = X≥2A`, `mw −= lo`, `mw += hi` | 1 | `ct_lt`, `select`, `add` | — | estimate within ±1 (W5d) |
| 23 | Big undershoot `mb = sign·(qtop≥2 ? qtop−1 : 0)` | 1 | `ct_lt`, `select` | — | `≤ qtop` |
| 24 | `m_j = big ? mb : mw` (the unused path is dropped here) | 1 | `select` | — | — |
| 25 | Coordinates: `v_r −= (m0·u_r ∓ m1·u_r')·2^s` | C×1, shift | — | `mul_word`, `shl_secret` (marks if a set bit leaves) | `≤ 4√(2Tmax·M0)` (W4) |
| 26 | Carried Gram: `r = h − (m·A)·2^s`, `C' −= (m·(h+r))·2^s` | W×1, shift | — | `mul_word`, `shl_secret` | `|m2ˢA| ≤ 1.5M0`, `|m(h+r)2ˢ| ≤ 2.25M0` (W3) |
| 27 | Swap if `C' < A'`: 8 coordinates, `A'↔C'`, `h1' ← −h1'` | C, W | `ct_lt`, `swap`, `select` | — | — |
| 28 | Consumer form `A'/2`, `C'/2` | W | `shr(1)`, exact (`A'` even) | — | — |
| 29 | `mark_if(!ok)` before the signature exits | any | none | new `mark_if` | — |
| 30 | Decode guard `N4(vₛ)² ≤ p·(1024N)²` | about 2×(encoding bits) + L + 20 | `from_le_bytes`, `mul`, `ct_lt` | — | width from the encoding; bytes that do not fit mark, and that reads as "invalid key" |

### 2.3 What sign-magnitude changes compared with the research code

The research crates use two's complement. In `prism-int`:

- The exp64 window estimate can use **magnitude** shifts toward zero (`shr` as it exists), with the sign applied afterwards. PR #45 2.3's "floor shift on signed values" is not needed, because the ±1 correction from `X` absorbs the difference (W5d). This removes one shift variant.
- Hensel division works on magnitudes and puts the sign back. Exactness is the same test.
- Conditional negation of `h1'` on a swap is `select(h, h.neg(), bit)`, and `neg` never overflows.
- The "signed bits + 1" of `bounds_ng.py` becomes plain magnitude bits. W and C come out the same at all five sets.

### 2.4 W5: the word-level bounds of the shared division (proof)

Let `b = bl(A)` and assume `|h_j| < 2^(b+e)` with `e` as computed.

- **(a) Leading word.** If `b ≥ 64`, then `k = b − 64` and `n = A >> k ∈ [2^63, 2^64)`. If `b < 64`, then `k = 0` and `n = A < 2^64`.
- **(b) Two-word numerator.** On the window path (`s = 0`), `|h| < 2^(b+60)`. On the big path, `s = e − 60` and the larger `|h| < 2^(b+e)`. Either way `num = |h| >> (k+s) < 2^(b−k+60) ≤ 2^124`.
- **(c) Division precondition.** If `b ≥ 64`, then `hi = num >> 64 < 2^60 < 2^63 ≤ n`. If `b < 64`, then `num < 2^(b+60)`, so `hi < 2^(b−4) < 2^(b−1) ≤ n`. So the fixed 64-iteration division never needs to saturate, and `qtop ≤ num/n < 2^61`. The `div_word` mark (`hi ≥ n`) is therefore unreachable, and a mantissa fits in one `SecretInt<1>`.
- **(d) The estimate is within one.** Let `t = |h|/A`. Since `n ≤ A/2^k < n + 1` with `n ≥ 2^63` (or `k = 0` exactly), `t − 2^−63 < num/n < t(1 + 2^−62) < t + 1/2` for `t < 2^61`. So `qtop ∈ [⌊t⌋ − 1, ⌊t + 1/2⌋]`. The half-up target is `⌊t + 1/2⌋` for positive `h` and `−⌈t − 1/2⌉` for negative `h`, and both are within one of `±qtop`. The correction `X = 2(h − mA) + A` with `0 ≤ X < 2A` then gives exactly the target, both half ties included.
- **(e) The discarded path.** In a big step the window path uses `qtop` computed at shift `k+s`, so `|mw·A| ≤ |h|/2^s + A ≤ 2M0`, and `X` stays below `9M0` (W2). In a window step the big path computes `qtop − 1` with `s = 0`, which is no larger. So **the path that is thrown away never marks**.

  The selection also makes this robust. `select` drops the mark of the operand it does not pick, and `big` is computed from bit lengths only, never from the discarded values. So even an unexpected overflow on the discarded path could not hide or fake one on the path that is used.
- **(f) Shift counts.** Every shift count `k+s` lies in `[0, 64W)`.

`check_exp64_words.py` asserts (a) to (f) on 200,440 random and edge-case operand pairs at all five Gram widths, including exact ± half ties, the window edge `e = 60/61`, `A < 2^64` and powers of two.

### 2.5 The overflow mark on admitted inputs

The mark is set only by a result that does not fit, or by `mark_if`. Rows 1 to 30 bound every result an admitted input produces below its buffer: W1 to W5 and the constructor bounds of `widths.py`. The modular operations (`mul_low`, `truncate`) are defined not to mark. Hensel's intermediate value is therefore never "overflow", and only the product-back comparison decides integrality. A failed integrality check sets `ok = 0`, not the mark (row 7 fits either way), and only `mark_if` turns it into a mark, at the signature boundary. **On admitted inputs no operation of 2c sets the mark.** That is the U obligation for 2c (Q5's last open item, O33), given the class bounds of 1.1.

### 2.6 Scratch ownership, erasure and peak stack

| Buffer | Width | Owner | Lives | Erased |
|---|---|---|---|---|
| `uₛ, vₛ, N` (and an optional `N⁻¹`) | C, W | `AdmittedKey` (the signer's caller) | key lifetime | `AdmittedKey` drop (prism-int wipes on drop) |
| `T`, `T⁻¹` | W | `ReduceState` | one call | `ReduceState::wipe` at hand-off, and drop |
| Constructor numerators (`n`, `x²`, `y²`, four numerator Gram entries, `A'C'`, `h²`, product back) | 2W | constructor frame (stack locals) | constructor | each `SecretInt` is wiped on drop at return |
| `U, V`, carried Gram | C, W | **caller-provided** `&mut ReduceState`, written in place | constructor to consumer | consumer's drop of `Reduced`. The reducer swaps in place and never moves them. |
| Per-step values (abs, windows, counts, mantissas, `X`, products, shifted terms) | W, C, 1, 2 | step function frame | one step | drop at the end of each step |
| Operation intermediates | N | inside `prism-int` (`Scratch`) | one op | `Scratch` drop |
| Status bit `ok` | bit | `AdmittedBasis`, then `Reduced`, then the signer | one signature | folded by `mark_if`, then wiped on drop |
| Consumer form `F` | W | `Reduced::form()` (derived on demand) | stage 3 | drop |

**What this does not cover** (2.1.3, "Wiped on drop"): the residue that by-value returns leave on the stack, and register spills. There are two proposals:

1. Add in-place forms of the hot-loop operations (`sub_assign_shifted`, `mul_word_into`) so per-step results are written into `ReduceState` and not moved.
2. After signing, call a `#[inline(never)]` safe "stack scrub" that zeroizes a local array of the measured peak plus a margin, from the same frame depth.

Both are to be measured under G4/O6. Without `unsafe`, neither is a guarantee.

**Peak stack (census estimate from `widths.py`, bytes; a `SecretInt<N>` is 8N + 8):**

| Set | `ReduceState` | Per-step peak | Reducer total | Constructor (general) total | **Peak** | Same at single width `LIMBS` |
|---|---:|---:|---:|---:|---:|---:|
| P248 | 1,152 | 1,232 | 2,576 | 4,160 | **4,160** | 6,944 |
| P324 | 1,440 | 1,472 | 3,152 | 5,216 | **5,216** | 8,736 |
| P376 | 1,632 | 1,632 | 3,536 | 5,920 | **5,920** | 10,016 |
| P500 | 2,112 | 2,032 | 4,496 | 7,680 | **7,680** | 12,832 |
| P664 | 2,688 | 2,528 | 5,664 | 9,792 | **9,792** | 16,928 |

The constructor, at double width, sets the peak at every set. These figures count only named values. Moves and spills can add 1.5 to 2 times. **Proposal:** a stack budget of 32 KiB at P664 for the scrub. The prototype measures the real peak by stack painting in a test harness (a separate binary, since the crate forbids `unsafe`) or with `-Z emit-stack-sizes`, and CI holds the result to the budget.

### 2.7 Missing prism-int primitives (proposed signatures)

All work depends only on `N` and public counts. Each new primitive needs vector lines at every width it is used at (1, 2, 11/14/16/21/27, 22/28/32/42/54). That means one regeneration of `tests/data/vectors.tsv` (3.3 MB, 2.1.6), batched.

```rust
impl<const N: usize> SecretInt<N> {
    /// Bit length of the magnitude; 0 for 0. Fixed work: per-limb lzcnt by masks.
    pub fn bit_length(&self) -> SecretInt<1>;
    /// self · 2^count for a secret count: limb barrel (⌈log2 N⌉ masked stages), then each limb
    /// times a masked power of two 2^(count mod 64) with carry (Rick's six-mask construction).
    /// Marks if a set bit leaves, or count < 0, or count ≥ 64N with a nonzero value.
    pub fn shl_secret(&self, count: &SecretInt<1>) -> Self;
    /// Magnitude / 2^count toward zero, sign kept (as `shr`). Marks if count < 0.
    pub fn shr_secret(&self, count: &SecretInt<1>) -> Self;
    /// self · scalar for a secret signed one-limb scalar: N limb products. Marks on overflow.
    pub fn mul_word(&self, scalar: &SecretInt<1>) -> Self;
    /// Exact width change. Marks if M < N and a dropped limb is nonzero.
    pub fn resize<const M: usize>(&self) -> SecretInt<M>;
    /// |self| mod 2^(64M), non-negative. Never marks (modular, for Hensel).
    pub fn truncate<const M: usize>(&self) -> SecretInt<M>;
    /// |self|·|rhs| mod 2^(64N), non-negative. Never marks (modular, for Hensel/Newton).
    pub fn mul_low(&self, rhs: &Self) -> Self;
    pub fn abs(&self) -> Self;
    pub fn is_negative(&self) -> SecretBit;
    pub fn is_odd(&self) -> SecretBit;
    /// The low k ≤ 64 bits of the magnitude are zero (public k).
    pub fn low_bits_zero(&self, k: u32) -> SecretBit;
    /// Sets the mark when the bit is set (monotone: never clears). Broadens "overflowed"
    /// to "overflowed or invalidated"; documented as such.
    pub fn mark_if(&mut self, bit: &SecretBit);
}
impl SecretInt<2> {
    /// ⌊self / d⌋ by a fixed 64-iteration restoring division (no `div` instruction).
    /// Marks if d = 0 or the high limb is ≥ d (the quotient would not fit one limb).
    pub fn div_word(&self, d: &SecretInt<1>) -> SecretInt<1>;
}
impl SecretBit {
    pub fn and(&self, rhs: &Self) -> Self;  // marks: OR of both
    pub fn or(&self, rhs: &Self) -> Self;
    pub fn not(&self) -> Self;
}
```

Optional: `mul_widen::<M>()` with a const assertion `M ≥ 2N`. It would make the constructor's products cost `N²` instead of `(2N)²`, but the constructor is about 0.2% of the total, so it is not needed.

Hensel division, the Newton inverse, the exp64 step and the form conversion belong in the signer's 2c module, built from these primitives. They do not belong in `prism-int`.

**CT-barrier dependency.** Every mask in these primitives goes through `subtle`'s `Choice` today. Its barrier is `read_volatile`, which costs 7 to 12% and is not audited by Shekyl (`ct-barrier-policy`). Section 0 already permits one audited `prism-ct-barrier` crate. Whether `prism-int` moves onto it **before** `shl_secret` and `shr_secret` are ported is Rick's call. The secret-amount barrel shifter is exactly where the weak barriers broke. The secret shift is the first primitive to port and inspect: release assembly scan, control-flow traces on x86-64 and AArch64, Memcheck with the known-leaky positive control, then timing. **Real AArch64 hardware is required**, because the earlier ARM64 traces ran under QEMU.

---

## 3. Consumer acceptance test

### 3.1 The exact conversion

The carried normalized Gram is `A' = N4(U)/(2T)`, `C' = N4(V)/(2T)`, `h0' = D(V,U)/(2T)` and `h1' = D(V,JU)/(2T)`, with `det = A'C' − h0'² − h1'² = p`.

The consumer form of PR #45 2.3, on columns `(U, JU, V, JV)`, is `Fᵢᵢ = Wᵢᵢ/(4T)`, `Fᵢⱼ = 2Wᵢⱼ/(4T)` for `i > j`, and zero above the diagonal.

The numerator Gram has `W(U,JU) = W(V,JV) = 0`, `W(JU,JU) = W(U,U)`, `W(V,U) = h0`, `W(V,JU) = h1`, `W(JV,U) = −h1` and `W(JV,JU) = h0`. The `W(JV,U)` entry follows from `D(JV,U) = −(v₁u₀ − v₀u₁ + p(v₃u₂ − v₂u₃)) = −h1`. So:

```
F = [[ A'/2,   0,    0,    0  ],
     [  0,    A'/2,  0,    0  ],
     [ h0',   h1',  C'/2,  0  ],
     [ −h1',  h0',   0,   C'/2]]
```

- **Diagonal factor:** the normalized diagonal is twice `F`'s. `A'` and `C'` are even because `N4/(4T) = nrd/T` is an integer, so the halving is an exact public shift.
- **Off-diagonal convention:** `F` stores the doubled numerator entry over 4T, which equals the normalized projection itself. Row 3 is `(−h1', h0')`.
- **The C reference** (`quat_lideal_reduce_basis`) returns `4F` on the same basis. Differential tests across different bases need the change of coordinates first (PR #45).

### 3.2 Test specification

For each fixture, run the production reducer at its scheduled count and check:

1. **Lattice:** each returned column lies in the input lattice, by Cramer's rule against the input columns, and `|det|` is unchanged.
2. **Fresh against carried:** `F` built from the returned coordinates (`form_from_basis`) equals `F` converted from the carried Gram. This catches drift in the carried update.
3. **Identity:** `4T·cᵀFc = N4(Bc)`, that is `cᵀFc = nrd(Bc/2)/T`, on the four units, the six pair sums (which fix every coefficient of a quadratic form) and three fixed mixed vectors.
4. **Terminal state, read off `F`:** `F00 ≤ F22` and `−F00 ≤ F2j < F00` for j = 0, 1. Only row 2 carries information, since row 3 follows from it.
5. **Early-stop control:** the same input at `steps = 1` must **fail** (4), and (3) must still pass. The identity holds for any basis, so it can never catch an early stop on its own. The reducer takes `steps` as data (section 4), so this control uses the production function itself.
6. **Exact output** equals the oracle's pair for the fixture (`reduce_exact` of `scripts/stage2c_oracle.py`). This is what catches a non-strict swap, initial or at equal norms: the output is still reduced but differs.
7. Then PR #45's other toy cases (`test_ties_and_order`, the exhaustive minima) and the P500 saturation fixture.

### 3.3 The p=3, T=5 fixture, worked by hand

The fixture is PR #45's `test_final_size_reduction_after_last_swap`: `U = (10,0,0,0)`, `V = (4,9,1,0)`.

**Admission.** T = 5 is odd, x = 4 is even and y = 9 is odd, both in `[0,10)`. Then `n = 16 + 81 + 3 = 100`, `4 | 100`, and `100/4 = 25 = 5·5`, so the division by T is exact. The normalized Gram is `(A', C', h0', h1') = (10, 10, 4, 9)`, and `det = 100 − 16 − 81 = 3 = p`. In numerator units `(A, C, h0, h1) = (100, 100, 40, 90)`.

**Ordering.** `N4(V) = N4(U) = 100`. The swap is strict, so there is none.

**Step 1.** `A = 100`, `h0 = 40`, `h1 = D(V, JU) = D((4,9,1,0), (0,10,0,0)) = 90`. Then `m0 = ⌊0.4 + 0.5⌋ = 0` and `m1 = ⌊0.9 + 0.5⌋ = 1`. `V ← V − JU = (4,−1,1,0)` with `N4 = 16 + 1 + 3 = 20 < 100`, so the vectors swap: `U = (4,−1,1,0)`, `V = (10,0,0,0)`. In normalized terms `(A', C', h0', h1') = (2, 10, 4, 1)`.

**After one step**, `N4(U) = 20` is already the minimum. Exhaustive search over coefficients in `[−4,4]⁴` gives λ₁ = 20, so a shortest-vector-only test passes here. But `F` is:

```
F = [[1,0,0,0],[0,1,0,0],[4,1,5,0],[−1,4,0,5]]
```

`F20 = 4 ≥ F00 = 1`, and `F21 = 1 = F00` is the +1/2 tie, outside `[−F00, F00)`. **The terminal test fails**, while the identity holds, as it does for every basis.

**Step 2.** `A = 20`, `h0 = D((10,0,0,0), (4,−1,1,0)) = 40` and `h1 = D((10,0,0,0), (1,4,0,1)) = 10`. Then `m0 = ⌊2 + 0.5⌋ = 2` and `m1 = ⌊0.5 + 0.5⌋ = 1`: the **+1/2 tie rounds up**. `V ← (10,0,0,0) − 2(4,−1,1,0) − (1,4,0,1) = (1,−2,−2,−1)` with `N4 = 1 + 4 + 3·5 = 20`. That is not below 20, so there is no swap.

**Step 3** (the fixed point). `h0 = 4 + 2 − 6 = 0` and `h1 = 1 − 8 − 3 = −10`, so `m1 = ⌊−0.5 + 0.5⌋ = 0`: the **−1/2 tie rounds to 0**, and nothing changes. The two active steps contain one swap.

**Result.** The carried Gram is `(2, 2, 0, −1)`, and

```
F = [[1,0,0,0],[0,1,0,0],[0,−1,1,0],[1,0,0,1]]
```

This is the same `F` whether it is built fresh from the coordinates or converted from the carried Gram. Check `c = (0,1,1,0)`: `Bc = JU + V = (1,4,0,1) + (1,−2,−2,−1) = (2,2,−2,0)` and `N4 = 4 + 4 + 12 = 20`, so `nrd(Bc/2)/T = 5/5 = 1`. On the other side, `cᵀFc = F11 + F22 + F21 = 1 + 1 − 1 = 1`.

The terminal test passes: `−1 ≤ 0 < 1` and `−1 ≤ −1 < 1`. The C reference would return `4F`.

**What the fixture catches:**

- an early stop after the first swap;
- +1/2 rounded toward zero, which gives `m1 = 0` in step 2 and a non-reduced output;
- a non-strict swap at the start, or at step 2's equal norms. Either one changes the output pair (check 6);
- a sign error in `F30`, which breaks the identity at `c = (1,0,0,1)`.

`check_fixture.py` asserts every number above (`results/fixture.txt`). It also sweeps every generic HNF at 13 small primes and odd T < 60, at every schedule length from 0 to 11: 143,064 runs. In all of them, the converted `F` equals the fresh `F`, the identity holds, and the `F`-based terminal test agrees with the coordinate terminal test. The mutants (non-strict initial swap, non-strict swap and toward-zero ties) are asserted to change the output. Negative controls (odd x, even y, 4T not dividing, even T, a non-member and a proper sublattice) are all refused by the constructor model.

---

## 4. The schedule as data, and the perf protocol

### 4.1 API

```rust
#[derive(Clone, Copy)] pub enum ScheduleTable { Pr45Reviewed, JointCandidate }
pub const fn steps(set: ParamSet, class: InputClass, table: ScheduleTable) -> u32;
// reduce() reads class from the AdmittedBasis, so a caller cannot pair an input with another row.
```

- The step count is public and comes from public data only: the set, the operation, the key format and the table. One reducer loop runs `for _ in 0..steps`, so both tables execute the same machine code with a different trip count.
- The signer's public entry points pin `Pr45Reviewed`. `JointCandidate` is reachable only from tests and benchmarks (`#[doc(hidden)]`, or a non-default feature) until the independent certificate review (running separately on `experiment/certificate-review`) and Rick's decision.
- Tests: PR #45's `test_native_schedule_table` already ties `Pr45Reviewed` to the spec table. Add the same test for `JointCandidate` against `tighter-progress-certificate/results/step_counts_new.csv`. Rick's `df92507` reproduces those counts.

| exp64 steps | P248 | P324 | P376 | P500 | P664 |
|---|---|---|---|---|---|
| Generic signing: PR #45 / joint | 312 / 280 | 401 / 360 | 463 / 415 | **609 / 546** | 803 / 719 |
| Guarded pre-reduced, 256 | 297 / 267 | 387 / 347 | 448 / 402 | **594 / 533** | 787 / 706 |
| Keygen, norm D | 231 / 208 | 360 / 323 | 344 / 308 | 459 / 411 | 712 / 638 |
| Secret-ideal pre-reduction | 24 / 21 | 25 / 22 | 26 / 22 | 26 / 22 | 29 / 23 |
| Challenge alone | 220 / 197 | 287 / 257 | 331 / 297 | 442 / 396 | 587 / 525 |

### 4.2 Perf protocol: what the shorter schedule buys at P500

1. **One binary.** A release build of the prototype with `cargo +1.97.1`, fat LTO and one codegen unit, at one commit. The step count is passed as a runtime `u32` from the command line, so the compiler cannot specialize either count. Record the toolchain, flags and binary hash.
2. **Correctness gate first.** On every input, both counts must give identical final bases and logical-trace hashes, equal to the exact oracle. Both counts are at least the steps needed, so the extra steps must be no-ops. Any difference stops the run.
3. **Inputs.** The same files for both counts:
   - 60 honest guarded pre-reduced signing intersections (the class the native signer uses, per `a80245c`);
   - 60 honest generic-HNF signing lifts;
   - the crafted Q5 and k=1 inputs.
4. **Counts.** `{609, 546}` on generic inputs and `{594, 533}` on guarded inputs, as Rick asked, plus `{0, 1, 273}` to fit `time = setup + steps·per_step + erasure`. The fit gives a predicted saving of 63 or 61 steps times the per-step cost. The measurement should match `546/609 ≈ 0.897` of the reduction part.
5. **Timed region.** Each phase is timed separately and summed:
   - construction, with a cold `T⁻¹`; the warm `N⁻¹·q⁻¹` variant is reported only if it is adopted;
   - reduction;
   - erasure: an explicit wipe of `ReduceState` and the drop of `Reduced`, plus the stack scrub if it is adopted.
6. **Method.**
   - Pin one core, with turbo and frequency scaling off or recorded.
   - Read the TSC with `lfence; rdtsc … rdtscp; lfence` on x86-64, or `prism-bench`'s counter, which has its own exception. On AArch64 use `cntvct_el0` or PMU cycles.
   - Run 10 warm-ups, then 5 processes × 201 repetitions, with the order of the two counts randomized in every repetition.
   - Report the median of the five process medians for each phase, the ratio with a 95% bootstrap interval, and per-input maxima.
7. **Machines.** Rick's i9-11950H and a **real AArch64** machine, on the same commit. This is separate from the G7 constant-time evidence (dudect or trace comparison), which uses the same binary.

---

## 5. Open decisions for Rick

1. **Failure handling.** Adopt Option D: declassify at keygen and decode (two rows, I/U), fold into `mark_if` in signing, and broaden the meaning of the mark.
2. **What the signer does on `Overflowed`** at its exits: abort, or draw again (stage 5). Either way, no retry counter.
3. **Split widths against 2.1.2's one-width rule.** `GRAM = COORD = 11/14/16/21/27` and `CTOR = 22/28/32/42/54` per set.
4. **Which signing class is native.** The guarded intersection with the general constructor (594/533 at P500) or the generic HNF (609/546). This decides the headline perf row, and whether HNF shape is a failure-budget item.
5. **Runtime `A', C' ≤ M0` check** in the constructor (defence, two `ct_lt`).
6. **`N⁻¹` cache.** Not caching is recommended: the cold `T⁻¹` is about 0.2% and keeps ownership simple, which matches Rick's `a80245c` note against extra key-derived state.
7. **O0-stability check at decode** (1.6): its method, owned by 2b/2d.
8. **The `prism-int` additions** (2.7), batched into one vector regeneration.
9. **CT-barrier policy.** Whether `prism-int` moves from `subtle` to the audited `prism-ct-barrier` before the secret shift is ported.
10. **Real AArch64 hardware** for traces, timing and the perf protocol. The secret shift is the first primitive to port and inspect.
11. **Joint schedule adoption** (546 at P500), after the independent certificate review. PR #45 keeps 609 until then.
12. **Stack scrub and budget** (32 KiB at P664, measured).

## 6. Reproduction

From `research/stage2c-design-notes`, with Python 3 (stdlib only):

```
python3 -B check_fixture.py            # section 3: p=3, T=5 by assertion + 143,064-run sweep (~10 s)
python3 -B widths.py --csv results/widths.csv   # sections 2.1 and 2.6
python3 -B check_exp64_words.py        # section 2.4: W5 (a)-(f), 200,440 operand pairs
```

| File | Contents |
|---|---|
| `s2c_model.py` | Exact contract, the normalized constructors (Hensel plus product back), the carried-Gram run, and the form conversion and checks |
| `check_fixture.py` | The consumer acceptance test of section 3 on the model |
| `widths.py` | Bounds, limbs per buffer and per set, and the stack census |
| `check_exp64_words.py` | Word-level bounds of the shared-division exp64 step on sign-magnitude values |
| `results/` | The outputs above |
