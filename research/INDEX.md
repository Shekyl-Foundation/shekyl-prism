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
