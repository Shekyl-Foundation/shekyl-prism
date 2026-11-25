# Research index

This is the long-lived `research` branch of `shekyl-prism`. It collects
public experiment work that is kept off `dev` and is never merged into
it. PRISM is the signature scheme of ePrint 2026/443 (Andrea Basso,
Giacomo Borin, Wouter Castryck, Maria Corte-Real Santos, Riccardo
Invernizzi, Antonin Leroux, Luciano Maino, Frederik Vercauteren, and
Benjamin Wesolowski). Shekyl's `shekyl-prism` is an implementation of
it. None of the harnesses here is part of a crate, none is built by
CI, and none makes a constant-time or security claim.

The Shekyl parameter sets, and the only ones that exist, are P248,
P324, P376, P500 and P664, as defined in `prism-field`. P363 and P384
are not Shekyl parameter sets.

## Experiments

| Folder | Question | Method | Verdict | Source |
|--------|----------|--------|---------|--------|
| [`cornacchia-harness/`](cornacchia-harness/) | Does masking a 2-D lattice basis with a random unimodular matrix (ePrint 2023/807 Alg. 10) give a constant-time Cornacchia? | Instrumented classical Cornacchia versus Lagrange reduction of a masked 2-D lattice, on prime streams at PRISM-sized inputs. ePrint 2023/807 was read from indexed excerpts (the PDF was blocked); citations must be checked against the paper before use. | No. The masked trace equals the unmasked lattice trace plus independent noise. A public seed hides nothing. Constant time needs a fixed-iteration algorithm, not a per-input mask. | `experiment/cornacchia-ct-lattice-harness` @ [`8eabf86`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/8eabf863edf2ab73597d83b292e27230201cbe0a) |
| [`latticeprism-4d/`](latticeprism-4d/) | Does a hash-expanded 4-D embedding of primes, seeded from H(pk), make exact LLL target-independent and size-blind? | Instrumented exact LLL on 4-D left ideals of norm q (H1: mask a fixed ideal's basis; H2: the XOF picks which of the q+1 ideals), plus a simplified secret-product panel, on the five Shekyl sets. | H1 keeps a strong target dependence. H2 flattens target dependence to chance, but is not constant time: exact LLL still costs about 1.5 steps per bit of q (about 2.0 on the secret product). A public seed hides nothing. Only a fixed schedule gives a constant trace. | `experiment/cornacchia-ct-lattice-harness` @ [`8eabf86`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/8eabf863edf2ab73597d83b292e27230201cbe0a) |
| [`latticeprism-seeding/`](latticeprism-seeding/) | Which H2 seed is deterministic and uniformly distributed, without being publicly precomputable? | The same H2 selector, compared under public baselines (B0, Hpk) and secret constructions (Hsk, COM, EPH, REV). | Deterministic and uniform can only coexist as pseudorandom-from-secret. Hsk, COM and REV meet that; B0 and Hpk are fully precomputable, and EPH leaks its seed in the signature. COM, EPH and REV change the signature format and are not for adoption without a design round. No construction is constant time. The scheme already has the commitment property in its salt. | `experiment/cornacchia-ct-lattice-harness` @ [`8eabf86`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/8eabf863edf2ab73597d83b292e27230201cbe0a) |
| [`salt-reshaped-lattice/`](salt-reshaped-lattice/) | Can the scheme's salt reshape the 4-D grid so that every lattice has one standard covolume and exact LLL no longer scales with the challenge? | Salt in the challenge hash, as in the scheme. Compared H2, D0, exact scaling, cofactor padding, the unique constant-determinant superlattice, and 256-bit Gram quantization, on I_q and on I_sec · I_q. | No. Exact LLL is scale-invariant, so a homothety leaves the trace identical. Padding flattens the count only by making every lattice large. The superlattice no longer solves the problem. Quantized precision was sometimes flat where it worked, and failed most of the time: a hint, not a method. Only a fixed schedule flattens an exact reduction. | `experiment/salt-reshaped-lattice` @ [`4b946b9`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/4b946b928568380eaf347ba9649a9e5c5e328399) |
| [`higher-dim-lattice/`](higher-dim-lattice/) | Does raising dimension — or any other single lattice characteristic — change the steps-per-bit law or flatten the trace? | One-characteristic-at-a-time ablation against 4-D H2: dimension 6–16, a direct sum, covolume padding, starting skew, the Euclidean form, 256-bit precision, a weaker LLL delta, selection (H1, D0), and ignore-the-problem controls. | No. Extra dimensions add a fixed overhead per axis and leave the 1.5 steps/bit growth untouched. A worse basis or form costs more or varies more. A weaker delta changes nothing material. Padding again flattens only by making every lattice large (3.26× at P248). The 2-D→4-D flattening was fresh-lattice selection (H2), not dimension, and it does not remove the size slope. | `experiment/higher-dim-lattice` @ [`54dbc9f`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/54dbc9f15bcabc129403ba5e4461b95183f102df) |
| [`fixed-schedule-reduction/`](fixed-schedule-reduction/) | Does a reduction with a schedule set by the parameter set, or a fixed-budget enumeration or sieve, flatten the exact-LLL count without leaking bits of the secret norm N? | Exact LLL versus a fixed-tour 4-D reduction and Z[i]-Lagrange with a fixed step count (plus 8–64-bit quotients), and a shortened pre-reduction plus a fixed-budget Schnorr–Euchner enumeration or a pairwise sieve. Tour counts sized from B = ⌈L/2⌉+32, never from N. 400 keys × 10 messages with varied bits(N); a 3,000-trace fixed-versus-random-key Welch t-test. D0 excluded; H2 with a secret seed. | Yes. Exact LLL leaks about 2.0–2.1 steps per bit of N (r = 0.31–0.49 on 4,000 traces; a t-test flags a key whose N-length sits away from the typical value). A fixed schedule sized from B has one operation trace per set (CV 0) and does not track bits(N). Z[i]-Lagrange is the cheaper correct method: λ₁ on every sample, including at B, at about 0.28× exact LLL's multiply-and-divide count. A generic 4-D up-down tour also works, at about 0.80–0.89×, with the same λ₁ rate as exact LLL. A 16- to 64-bit quotient is enough; 8 bits is not. A fixed-budget search can cover a few missed tours but cannot replace a large cut of the schedule. | `experiment/fixed-schedule-reduction` @ [`eea63ea`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/eea63eadfaee23f40bc96ba11eb738b02161f7a5) |

| [`lagrange-tour-bound/`](lagrange-tour-bound/) | Can the fixed-schedule Z[i]-Lagrange tour count be bounded provably from public parameter sizes instead of empirical maxima, and do bounded-precision quotients or Lehmer batching yield a provably smaller schedule? | Analytical step-recurrence and endgame proofs paired with exact GMP rational (`mpq`) interval branch-and-bound (`certify.py`), 300 PRISM ideals/set (`P248`–`P664`), explicit Rolletschek worst-case chains, 160,230 toy valid ideals, 8–64-bit truncated quotients, and `W=64/128` certified Z[i]-Lehmer batching. (Rolletschek 1986, Collins 2002, Vallée 1991, Napias 1996, arXiv:1806.03113/1904.05285 read from abstracts/excerpts.) | Yes—and the empirical schedule `T_emp` (189/239/283/359/465) is unsafe against worst-case inputs, which hit 276/356/411/542/714 steps under Rolletschek `2+sqrt(3)` chains. GMP-certified 2-step Lyapunov contraction (`rho_2 = 87/1000`, `<= 4` tail swaps for `lam >= 10^-3`) yields provable tour counts **301 / 387 / 446 / 587 / 773** (`0.47x–0.48x` exact LLL, within 8.2–8.8% of the Rolletschek lower bound). Bounded precision never saves tours and hits half-integer 2-cycles (fixable for `prec >= 16` with 1 exact cleanup step). Exact-certified Lehmer batching (`W=128, m=24`) runs in 12–30 batches (`~0.012x–0.015x` exact LLL) with zero fallbacks on random and Rolletschek inputs, though a small fixed batch count is not unconditionally provable against boundary-tie chains. | `experiment/lagrange-tour-bound` @ [`51e290c`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/51e290c252bdd22facebe250478b025d563e8cd7) |
| [`rust-lagrange-ct/`](rust-lagrange-ct/) | Does the fixed-schedule Z[i]-Lagrange reduction hold up as real constant-time code, in cycles and in timing tests? | Standalone Rust crate (outside the workspace and CI): fixed-limb integers sized from B, mask selects behind an inline-asm barrier, branch-free bit length and barrel shifter, fixed-iteration restoring division, certified T_prov = 301/387/446/587/773 plus one exact cleanup step (the empirical T as a secondary row). Cross-check with the Python model and the tour-bound adversaries (1,508 vectors per set). rdtsc cycles against exact LLL (GMP and num-bigint) and exact Z[i]. A dudect fixed-versus-random Welch t-test. Valgrind memcheck with secrets marked undefined (ctgrind), with positive controls. An assembly scan. | Yes. The CT code returns λ₁ on every vector, Rolletschek chains included, and matches the Python 64-bit model bit for bit. The empirical T fails the Rolletschek chains in Rust too. No leak: max \|t\| 2.93 over 220,000 measurements (threshold 4.5; controls 47–154), \|r\| with bits(N) ≤ 0.012, memcheck clean on all sets. No integer division or bit-scan in the compiled code. Cost: 0.19–0.23× exact LLL on GMP (0.08–0.11× num-bigint) in cycles, under half the model's 0.47× prediction. It is 4.8–7.8× variable-time exact Z[i], which is the price of constant time. A first `black_box` barrier compiled to a secret branch; it was found and fixed. Caveats: a noisy shared VM; rustc 1.85.1, not the pinned 1.97.1. | `experiment/rust-lagrange-ct` @ [`6faffe9`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/6faffe99ac5f36876a00939d099f27aaacb7e613) |
| [`fixed-schedule-norm-equation/`](fixed-schedule-norm-equation/) | Does `quat_qlapoti` (the norm equation in Section 5 signing and keygen) leak the secret, and does a fixed-schedule version carry the same advantage as fixed-schedule reduction? | Faithful Python port of `quat_qlapoti` and `prime_norm_reduced_equivalent` on O₀ (`PRISM_v2` `87fad7d`, cross-checked against compiled C at `lvl1/3/5`), 2,000–4,000 sigs + 1,000–2,000 dudect pairs + fixed small-ideal/per-ideal panels across P248–P664, O29 exponent what-if (`e−2`, `e−1`, `e`), and a fully fixed-schedule executor with trace hashing and analytical Gauss/Euclid bounds. | `bits(N)` does not leak directly (`|ρ| ≤ 0.032`, fixed-key dudect `|t| ≤ 1.59`) because shortest-equivalent normalization maps `I_sec · I_q` to `n ≤ √p`. Instead, the reference leaks the small ideal `S` (`ρ(work, v₂ n) = −0.12` to `−0.14`, fixed-small-ideal dudect `|t|` up to `6.4`, `2.5×–17.6×` spread in `p(S)` across ideals driven by `f`) and has four uncapped inner loops (`gen` draws 194, `λ`-skips 59,995, Gauss 92, Euclid 126). A fixed-schedule `quat_qlapoti` eliminates schedule variance (`1` distinct trace, `0/0` unfinished steps, `100%` valid solutions) but costs **39×–391×** mean work at 99.9% (`P = 2,662–37,528`; two-stage `23×–229×`) and **249×–2,513×** at `2⁻⁶⁴`. Raising target exponent `e−2 → e` (O29) lifts `M4>0` at P376 (`0.15 → 0.55`) and cuts passes `4.1×`. | `experiment/fixed-schedule-norm-equation` @ [`7a70ea8`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/7a70ea87e41bab27e4b2e906dd9aa11cbdbf4419) |

| [`keygen-precompute/`](keygen-precompute/) | Which secret-only PRISM signing work can move to key generation, what does it save per signature vs. secret-key growth, and does remaining work still need a fixed schedule? | Code/spec audit of `shekyl-prism` and `PRISM_v2` (`87fad7d`); PARI-verified `EXP_HNF` (HNF + `E_vk` canonical basis) and `EXP_RED_O0` (`Z[i]`-Lagrange pre-reduced `{u_sec, v_sec}` with `O₀`-coordinate intersection mod `q`); 500 setup keys, 400 varied-`N` × 10 + 100 boundary-`N` × 100 keys/set, and 3,000-trace `dudect` t-tests across P248–P664, evaluated under both empirical maxima and `lagrange-tour-bound`'s certified `T_prov`. | Storing `{u_sec, v_sec}` (`+20–24 B` compact, or `+212–524 B` with `E_vk` canonical basis for verify-after-sign) eliminates `quat_lideal_create`, `4×8` HNF, and the `bits(N)` leak (`exact LLL` slope `+1.89..+2.24 → −0.09..+0.13`, `dudect |t| 12.7–27.3 → ≤1.20`). With a keygen balance guard (`nrd(v_sec) ≤ 2√p·N`, 100% observed pass rate), `T_prov` drops 18 steps (`301/387/446/587/773 → 283/369/428/569/755`), cutting setup+reduce 64-bit limb ops by **2.15%–14.59%** vs. `BASE_SPEC` (**95.2%–95.3%** vs. `PRISM_v2`) with `0.11–0.41` signature break-even. `quat_qlapoti` on `S = shortest_equivalent(I_sec·I_q)` and `theta_chain` stay per-message. Remaining work still needs a fixed schedule (`t = 0`). Changing `SECRETKEY_BYTES` needs a design round; `EXP_RED_O0` changes stage 5 byte-for-byte replay (`EXP_HNF` preserves replay with negligible signing savings). | `experiment/keygen-precompute` @ [`649a26c`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/649a26cc12c30d90ff36975f7179682455c31315) |

| [`secret-ideal-rerandomization/`](secret-ideal-rerandomization/) | Does drawing a fresh equivalent secret ideal `J = I · β̄ / N` per signature remove the `bits(N)` leak from 4-D reduction (and `S` leaks in `quat_qlapoti`), and is it worth adding on top of the certified fixed `Z[i]`-Lagrange schedule? | Keygen `Z[i]`-reduction (`{u, iu, v, iv}`) + per-signature sampling (`plain`, `norm`, `gauss` × `prime`/`odd`, 20k sigs/set), 900 paired products/set (`I∩I_q` vs `J·K_J`) with map-back verification, 1,000-trace fixed-key-at-`B` vs random-key `dudect` under exact LLL and certified `Z[i]`-Lagrange (`T_prov = 301/387/446/587/773`), plus a `quat_qlapoti` re-randomization panel (`O₀` model). | Masks `bits(N)` under exact LLL (`1.78–2.20 → −0.21..+0.40` steps/bit; `dudect |t|` `11.6–19.7 → 0.09–0.99` with `odd_gauss`, `2.25–2.28` candidates mean, `K = 76–78` for `2⁻⁶⁴`; `100%` map-back agreement, no format change). However, certified `Z[i]`-Lagrange (`T_prov`) already runs in `1` trace (`t = 0.00`) at `0.464–0.475×` exact LLL with no extra bookkeeping, and re-randomization does **not** alter `n(S)` (`100%` invariant) or remove `S`-dependent `quat_qlapoti` leaks (`fixS |t| = 2.11–3.26` vs `2.21–3.64` baseline). Optional defence in depth only; not worth adding as primary protection. | `experiment/secret-ideal-rerandomization` @ [`a19677f`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/a19677f4108771413b3371131617241b424fa982) |

| [`ct-feasibility-gate/`](ct-feasibility-gate/) | How should `docs/SIGNER_SPEC.md` enforce constant-time feasibility before finalizing stages 2c and 3 while keeping G4 as the final hardening gate? | Design-review synthesis (`dev` at `26ce141`) mapped against `SIGNER_SPEC.md` (G1–G6, Q1–Q5, 2.1.3), `LEDGER.md` (O4–O34), and `research/INDEX.md` findings; drafted Section 0 and proposed ruling G7 in `docs/SIGNER_SPEC.md` plus `research/ct-feasibility-gate/NOTES.md`. | Drafted Section 0 (0.1–0.8) and proposed G7: requires secret-dependence classification, affordable CT strategy + cost estimates across P248–P664, certified bounds (`T_prov` + cleanup), Q5/Q4 proofs without overflow-driven retries, PR-time declassification review + sticky sensitivity, compiled-artifact checks (`x86_64` + `aarch64`, Rust `1.97.1`), non-sensitive transcript provenance, and three-way equivalence checks; leaves G2 `unsafe` barrier policy (`experiment/ct-barrier-policy`) and Stage 3/5 format choices open for Rick. Branch kept alive for PR review. | `experiment/ct-feasibility-gate` @ [`e91ca93`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/e91ca9335a12a653e988a8a970a2627a9838a609) |
| [`prime-certificates/`](prime-certificates/) | Can independently checkable primality certificates replace perpetual probable-prime retesting of the constants generator's fixed primes (ledger O19)? | PARI 2.17.4 Atkin–Morain ECPP for all 14 unique large primes (five field moduli, four unique secret norms, five item-19 cofactors); independent Python verifier (no `primecertisvalid` for the verdict); Lucas N+1 proofs for the field primes, where `p+1 = f·2^e` is completely factored; next-prime gap compositeness proofs for items 19 and 33; trial division of the 101 `small_primes` and 102 complementary 3-mod-4 primes. | Yes. All 14 ECPP certificates verify independently and with PARI; all five field primes have a second Lucas N+1 proof; all nine next-prime gaps are proved composite. Generation took 6.73 s; verification takes about 5 s with the standard library only. A proposed nightly step (in addition to SymPy) lives in `proposed-ci/` and is not applied to `dev`. O19 can be closed once the certificates are adopted on `dev`. | `experiment/prime-certificates` @ [`58582f8`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/58582f82d2a0e586856ab1752b74ae385b52303c) |
| [`q5-width-proof/`](q5-width-proof/) | SIGNER_SPEC Q5: what is the largest intermediate of the reduction actually proposed (certified fixed-schedule Z[i]-Lagrange: 64-bit quotients, one exact cleanup, conversion back, and the keygen pre-reduced variant), and does it fit the prism-int widths? | Proof that no step grows a norm by more than 2^-59.41 relative, clamps included, so every intermediate is at most 3·M0·(1+ε)^S; exact integer evaluation at all five sets (`bounds.py`); symbolic and randomised lemma checks; 22,412 instrumented runs a set (sampled ideals in the model order and O0 numerators, secrets at 2^B, Rolletschek chains, corner, ping-pong, tail and clamp-forcing bases, key generation, pre-reduced signing with sampled and crafted keys). | Q5 closed given Q4. Largest intermediate 2(B+2e−5)+5 bits = 1,303 / 1,681 / 1,947 / 2,565 / 3,385, against widths 1,664 / 2,112 / 2,432 / 3,136 / 4,160 (361 to 775 bits spare; P500 571). Every variant fits, any HNF shape included (66 to 111 spare). Observed maxima within 2 bits of the proof, no run over its bound. Without Q4 it overflows by 363 to 1,203 bits. LIMBS unchanged. Also found: a valid prime-norm ideal on which the 64-bit quotient clamp leaves the basis unreduced after T_prov plus cleanup (widths unaffected; honest inputs about 2^-110), and the keygen guard nrd(v_s) ≤ 2√p·N rejects about 22% of sampled keys (c = 256 rejected none). | `experiment/q5-width-proof` @ [`1013aad`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/1013aada55276817a8fc21471c383bc492e4a259) |
| [`qlapoti-redesign/`](qlapoti-redesign/) | What is an affordable constant-time strategy for `quat_qlapoti`: bounded inner loops, target exponent `e` versus `e−2` (O29), or a newer norm-equation construction (SQIsign 3.0 / Qlapoty)? | Per-λ sweeps of the reference structure (600 prepared ideals S/set × 8 generators × 4,000 λ at `e−2`, `e−1`, `e`), a Qlapoty-style prototype on the same S (odd n), a work-unit cost model sized for the worst S (u→1/√2, n_min 34 bits under the bound, generator-level reuse correlation), and 1,500 C runs of PRISM_v2 `87fad7d` (lvl1/3/5 × used power e−2/e−1/e, and with the θ postcondition off). ePrint PDFs blocked; survey from excerpts, the SQIsign 3.0 spec and the Qlapoti sources. | All four loops cap safely (Gauss/Euclid caps ≈2× every observed max; no skip loop with a public prime-λ list; a generator pool replaces the per-call draw loop). The cost is passes, not caps: capped one-stage designs cost **319/1,888×** at P500 (worse than the prior 132/847× once n_min and dead generators are honest). **O29:** `e` is correct with PRISM_v2's square-root chain (300/300 keygen+sign+verify at e and e−1, 0 chain warnings), 2.3–3× fewer passes at lvl5/lvl3, verifier unchanged, but **pk and signature bytes change** (stage 5 replay breaks; format unchanged). The θ postcondition is load-bearing (≈97% of runs abort on even d1 without it). Recommended (proposal): `e` + reference structure + prime λ + sieve < 2^12 + oblivious compaction: **P500 15.5× (99.9%) / 88.6× (2⁻⁶⁴)**, P248 11.1/64.5, P324 10.0/57.5, P376 34.6/198, P664 11.4/65.4 (model estimates). Qlapoty-style at `e` is ≈2.5× cheaper again at P500 (6.2/35.2) but covers odd n only; even n, the postcondition/chain choice and replay mode need a design round. | `experiment/qlapoti-redesign` @ [`d37ec55`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/d37ec55045a2f5c03130c5db0a2d32463567c5ed) |

## In progress

None.

## Conclusions

From Rick's review of 22 November 2026, of the salt-reshape and
higher-dimension experiments, and of the earlier LatticePrism work they
rest on, and from the fixed-schedule experiment that followed:

1. **No lattice reshaping makes exact LLL size-blind.** Scaling the
   lattice leaves the trace identical, sample for sample. Extra
   dimensions add a fixed overhead per axis and leave the growth rate
   untouched. Masking the basis, changing the quadratic form, or
   loosening the reduction costs more or changes nothing. Padding with
   a cofactor flattens the count only by making every lattice large
   (3.26 times the cost at P248).
2. **Only a fixed schedule flattens the count of an exact reduction.**
   That is the documented reason the hardening stage goes to
   fixed-schedule routines (TCHES 2025 dim-4 LLL; ePrint 2025/2192; a
   fixed-iteration half-GCD). This closes the "LatticePrism" line as a
   route to constant time. The negative result belongs in the write-up.
   The harness itself remains useful as a first filter at stage 7.
   The fixed-schedule experiment confirms the positive side: a
   schedule sized from B gives a correct short vector at a
   data-independent cost. Z[i]-Lagrange finds λ₁ on every sample,
   including at B, at about 0.28× exact LLL's multiply-and-divide
   count. A generic 4-D up-down tour also works, at about 0.80–0.89×,
   with the same λ₁ rate as exact LLL. A fixed-budget enumeration or
   sieve is only a thin safety margin, not a substitute for the
   schedule. The schedule must be run in full: stopping early re-opens
   the bits(N) channel.
3. **The cross-set slope is not a leak.** Within a set, bits(q) is
   constant, and the parameter set is public. The reports' "size leak
   across sets" is therefore not a secret channel.
4. **The secret norm's length is a real channel, and a schedule sized
   from B closes it.** The secret ideal's norm N varies in length from
   key to key (signer specification section 2.1: typical L/2+16, bound
   B = ⌈L/2⌉+32). Exact LLL on I_sec · I_q leaks 2.0–2.1 steps per bit
   of N (r = 0.31–0.49 on 4,000 traces of 400 keys; well above the
   detectable |r| ≈ 0.044). A dudect-style t-test on 3,000 traces flags
   a key whose N-length sits away from the typical value, and does not
   flag one that sits near it. The fixed schedules, sized from B, have
   one operation trace per set and do not track bits(N).
5. **D0 must not be used.** The deterministic smallest ideal is about
   three times cheaper, but the response ideal has to be uniform among
   the ideals of norm q; the proof and ledger O4 rest on it. A hash of
   the secret choosing uniformly (H2 with a secret seed) is
   acceptable; "smallest" is not.
6. **Statistical power was low on the reshaping panels, and is
   adequate on the N-length test.** Panels of 24 to 32 keys detect η²
   of roughly 0.08 to 0.15, so "no key leak seen" there means little.
   The fixed-schedule experiment used 400 keys and 3,000-trace
   fixed-versus-random-key t-tests; that is enough to see the
   bits(N) leak in exact LLL, and to see that a fixed schedule's
   trace does not distinguish keys (t = 0). A t-test on the real Rust
   code still belongs after stages 2 to 4.
7. **The retry loops are unmeasured.** The lattice of the challenge is
   public. The secret-touching work is key generation's reduction and
   its search for a prime-norm equivalent; signing's intersection and
   reduction; the norm equation's search (17-bit generators, 20,000
   steps, 400,000-pass budget, two uncapped inner loops); the
   translation to an isogeny; and curve arithmetic with secret scalars
   and kernels. Those retry loops are far larger channels than LLL's
   step count, and none of the experiments here measures them against
   the key.

8. **Provable Z[i]-Lagrange tour bounds replace the unsafe empirical
   maxima.** Empirical maxima `189/239/283/359/465` reflect typical
   PRISM ideals (~0.32 steps/bit), whereas explicit Rolletschek
   `2+sqrt(3)` chains reach `276/356/411/542/714`. GMP-rational
   interval certification gives unconditional worst-case bounds
   **`301/387/446/587/773`** (still `0.47x–0.48x` exact LLL). Bounded
   precision (`prec <= 64`) never shrinks the tour count and enters
   half-integer 2-cycles unless followed by an exact cleanup step;
   `W=128, m=24` Lehmer batching reduces work by ~20x (`12–30` batches,
   `0.012x–0.015x` LLL) on random and Rolletschek inputs, subject to
   handling boundary-tie fallbacks.

9. **The Z[i]-Lagrange schedule is constant time in real code, and
   cheaper than predicted.** A Rust port in fixed-width arithmetic, at
   T_prov plus one exact cleanup step, returns λ₁ on every test vector,
   Rolletschek chains included. It shows no leak in a dudect test
   (max |t| 2.93 over 220,000 measurements) or under memcheck. It costs
   about 0.2× exact LLL on GMP in cycles, against 0.47× predicted from
   operation counts. Constant time costs 5–8× over a variable-time exact
   Z[i]-Lagrange. Compilers can undo mask selects: the first value
   barrier became a secret branch. ctgrind and an assembly scan are the
   guard, and must be re-run under the pinned toolchain.

10. **Fixed-schedule padding does not carry over cheaply to `quat_qlapoti`.**
    `shekyl-prism`'s Section 5 signing uses `quat_qlapoti` (not
    `quat_represent_integer`). Shortest-equivalent normalization hides
    `bits(N)` (`|ρ| ≤ 0.032`, fixed-key dudect `|t| ≤ 1.59`), so the
    leak shifts to the prepared small ideal `S` (`v₂(n)` via `gcd(n, λ)`
    skips, fixed-small-ideal dudect up to `|t| = 6.4`, and up to `17.6×`
    variation in `p(S)` at `P376` where `f = 65` shrinks `M4 > 0` to `15–16%`)
    and four uncapped inner loops. A fixed-schedule `quat_qlapoti` gives
    one trace per set (`0/0` unfinished Gauss/Euclid steps) but costs
    **39×–391×** mean work at 99.9% (`23×–229×` two-stage) and
    **249×–2,513×** at `2⁻⁶⁴`. Bounding the four inner loops, resolving
    O29 (`e` vs `e−2`), re-randomizing the secret representative, or moving
    to SQIsign 3.0's norm-equation formulation beat brute-force padding.

11. **Keygen pre-reduction saves 2–15% of setup+reduction limb work and
    removes `bits(N)` skew, but still requires a fixed schedule and a
    format design round.** `mat_BAcan_to_BA0_two` is already stored at
    keygen, while `quat_qlapoti` operates on
    `S = shortest_equivalent(I_sec·I_q)` and cannot be precomputed.
    Pre-reducing `I_sec` to `{u_sec, v_sec}` (`+20–24 B` compact,
    `+212–524 B` with `E_vk` canonical basis for verify-after-sign)
    removes `bits(N)` skew from intersection/reduction, drops guarded
    certified `T_prov` by 18 steps (`301..773 → 283..755`), and breaks
    even in `< 1` signature. Remaining reduction still leaks `q` and
    class shape without a fixed schedule, and changing `SECRETKEY_BYTES`
    (plus Stage 5 C-reference byte-for-byte replay under `EXP_RED_O0`)
    requires a design round.

12. **Per-signature secret-ideal re-randomization (`J = I · β̄ / N`) works
    mathematically and hides `bits(N)` from exact LLL, but adds no
    timing gain over `T_prov` and leaves `quat_qlapoti`'s `S` leak
    intact.** Accepting odd cyclic-HNF norms (`2.25–2.28` draws mean,
    `K = 76–78` for `2⁻⁶⁴`) with Gaussian-then-round sampling (`odd_gauss`)
    eliminates both `bits(N)` (`dudect |t|` `11.6–19.7 → 0.09–0.99`) and
    basis-shape leaks while mapping short elements back 100% unchanged.
    Because `Z[i]`-Lagrange at `T_prov = 301/387/446/587/773` already
    has `1` trace (`t = 0`) at `0.464–0.475×` exact LLL, and
    `quat_lideal_shortest_equivalent(J · K_J)` collapses back to the
    exact same norm `n(S)` 100% of the time (`fixS |t| = 2.11–3.26`),
    re-randomization is optional defence in depth only.

13. **The signer's fixed primes can be proved, not just retested as
    probable primes.** Fourteen unique large primes (the five field
    moduli, four unique secret-ideal starting norms, and five item-19
    cofactors) have pinned Atkin–Morain ECPP certificates, checked by
    an independent Python verifier that does not call PARI's
    `primecertisvalid`. The field primes also have a short Lucas `N+1`
    proof, as ledger O19 already noted was available because
    `p+1 = f·2^e` is completely factored. The “next prime after `2^k`”
    half of items 19 and 33 is proved by a compositeness witness on
    every integer in the gap. The 101 `small_primes` and the 102
    complementary primes that are `3 (mod 4)` below 1237 are proved by
    trial division. Verification takes about five seconds and needs
    only the standard library. This does not change the signing
    protocol. A proposed nightly step is recorded under
    `research/prime-certificates/proposed-ci/` and is not applied to
    `dev`.

14. **Q5 is closed for the fixed-schedule Z[i]-Lagrange, given Q4.**
    A 64-bit step, clamps included, grows no norm by more than
    2^-59.41 relative. So every intermediate (coordinates, quotients,
    Hermitian products, norms, the cleanup's rounding, and the 4-D Gram
    after conversion) is bounded by its starting value. The largest
    intermediate is 2(B+2e−5)+5 bits, which leaves 361 to 775 bits
    spare under prism-int's LIMBS (571 at P500). The bound needs Q4:
    without it, it overflows by 363 to 1,203 bits. The T_prov
    certificate does not cover the 64-bit quotient: a crafted valid
    ideal stalls on the clamp. A sticky report or a wider quotient is
    needed before 2c relies on it. The keygen balance guard as written
    rejects about 22% of keys, and width does not need it.

The quantized-precision runs (Q256, PREC_256) were sometimes flat where
they worked, which hints that bounded-precision reduction is size-blind
— the direction the constant-time LLL literature takes. They failed two
times in three, or more, so that was a hint and not a method. In the
fixed-schedule experiment, a 16-, 32-, or 64-bit quotient in
Z[i]-Lagrange matched the exact-quotient output on every sample; an
8-bit quotient did not meet the 99.9% bar. That is a stronger hint,
still not a complete fixed-width LLL.

The model throughout is the index-4 order with simulated keys, and only
LLL is modelled. Step counts are not cycles.
