# qlapoti redesign: report

Date: 2026-11-24. Branch `experiment/qlapoti-redesign` (off `dev` @
26ce141). Research for Shekyl PRISM; PRISM is the scheme of ePrint 2026/443
(Basso et al.). **Every work figure is a model estimate** (Python model,
fsne-compatible work units, not cycles). The only measured counts are the
C reference runs in T6. Full tables: [`results/TABLES.md`](results/TABLES.md).
Nothing here is adopted on `dev`. The changes that alter keys or
signatures (exponent e, a new search) are proposals for a design round.

## Setup

- 600 prepared small ideals S per set (60 simulated keys x 10
  messages, same generation as the prior experiment), sets P248, P324, P376,
  P500 (Shekyl), P664.
- Reference structure (`collect.py`): per S, 8 reference generators,
  each swept over lambda = 1..4000 at target exponents e-2 (the pin), e-1
  and e, logging where every lambda stops; plus 64 generator searches per S.
- Qlapoty-style prototype (`collect_q.py`): same S (odd n only), 4 balanced
  alpha0 x 4000 odd lambda at the same three exponents.
- Fixed schedules are sized for the **worst S**:
  - rates are pooled over the top u-decile (u = n/sqrt p) and scaled to the
    Hermite bound u = 1/sqrt 2 with a power law fitted to the upper half;
  - M4 is sized from a public n_min 34 bits below the bound (the prior
    experiment used 8 bits, which does not meet 2^-64);
  - generator validity uses the worst S seen;
  - reuse correlation is modelled per generator: a block of M slots
    fails with probability mean_g (1 - rho_g)^M, so "dead" generators count.
- Padding factor = fixed-schedule work / the reference mean work at e-2
  (prior T8: P500 1.26e6 units).

## Sub-experiment 1: bounded inner loops (T3, T4)

Completion against each cap (all S pooled; worst S where it matters):

| loop | P500 observation | cap chosen | completion at cap |
|---|---|---|---|
| generator draws | 42% first draw, 99.91% within 64, max 144 seen; worst S validity 0.039 per draw (n even with many small primes; mean 0.42) | no per-call cap: a **generator pool** of D draws for the whole schedule, valid ones obliviously compacted into the B block slots (D sized by a binomial tail at the worst validity) | 1 - 2^-65 by construction |
| coprime-lambda skip | C=1: 91% of slots find a coprime lambda (worst S 43%); C=6: 99.9% (worst 90%) | C=6 if consecutive lambdas are kept; better, **no skip loop at all**: a public list of prime lambdas (a slot with lambda dividing n is wasted, at most omega(n) slots) | prime list: no loop |
| 2-D Gauss | max 65, mean 52 | T_lat = 104 (ceil(0.3932 nb)+3) | 100% observed |
| Cornacchia Euclid | max 98 (e-2), 101 (e) | T_corn = 204 / 206 (ceil(0.7202 mb)+3) | 100% observed |

All four loops cap safely: the Gauss and Euclid caps sit about 2x above
every observed maximum, and both have analytical bounds (prior experiment,
lagrange-tour-bound). The cost does not come from the caps. It comes from
the number of passes, and it depends on how the passes are organised:

| P500, padding factor (99.9% / 2^-64) | e-2 | e |
|---|---|---|
| prior "fresh" design (n_min 8 bits) | 132 / 847 | - |
| capped reference structure, skip C=6, M=32, every slot expensive | 319 / 1888 | 79 / 467 |
| prime-lambda list, M=32, every slot expensive | 226 / 1341 | 55 / 327 |
| prime list + sieve < 2^12 + oblivious compaction (two-stage), M=64 | **50 / 287** | **15.5 / 88.6** |

Other sets are in T4 (two-stage at e: P248 11.1/64.5, P324 10.0/57.5, P376
34.6/198, P664 11.4/65.4).

The capped one-stage designs cost more than the prior estimate, for three
reasons:
- the honest n_min (34 bits instead of 8) adds about 8% per expensive slot;
- worst-case u sizing costs about 0.64x in rate at P500 e-2;
- 33–45% of reference generators are "dead": every Cornacchia success they
  produce fails the theta postcondition. This makes lambda reuse
  correlated.

The two-stage design wins because only about 18% of slots survive the cheap
filters plus the sieve. The expensive stage (primality test, Cornacchia and
good-prime stripping; about 7,800 units) then runs on Q ≈ 0.23 P slots only.

## Sub-experiment 2: O29, e versus e-2 (T2, T6)

**The paper and the code.** PRISM (2026/443) needs only rational 2^e
torsion with e at most v2(p+1); it does not require e-2. SQIsign 3.0 fixes
e = f-2 "for technical reasons": its Isogeny22Chain builds the kernel from
2^(e+2)-torsion points ([4]T1, [4]T2) to avoid square roots. PRISM_v2's
`dim2id2iso` instead calls the theta chain with `extra_torsion = false` on a
kernel of order exactly 2^used. The last two steps then take square roots
("only used for signing"). That path accepts any used ≤ e. In the
"transformed" case (endtype 1/2) it already runs at used-1. So e-2 in
PRISM_v2 is a convention inherited from SQIsign, not a requirement of its
own chain.

**C reference, 100 seeds x lvl1/3/5 per variant (counts, not estimates):**

| variant | keygen / sign / verify ok | chain warnings | bytes = pin | mean qlapoti passes lvl1 / lvl3 / lvl5 |
|---|---|---|---|---|
| e-2 (pin) | 300 / 300 / 300 | 0 | - | 348 / 2952 / 1750 |
| e-1 | 300 / 300 / 300 | 0 | pk 0, sm 0 | 330 / 1787 / 1148 |
| e | 300 / 300 / 300 | 0 | pk 0, sm 0 | 353 / 1000 / 761 |
| e-2, postcondition off | 8 / 8 / 5 | - | - | aborts |
| e, postcondition off | 12 / 12 / 8 | - | - | aborts |

- **Correctness and verifier.** With the exponent at e, every key and
  signature verified, with no chain warnings. The verifier code does not
  change: it never sees the exponent of the signer's norm equation (the
  response is checked through the same isogeny data). The verifier cost
  does not change.
- **Signer cost.** The mean pass count falls 2.95x at lvl3 (P376) and
  2.3x at lvl5 (P500), and does not change at lvl1 (P248). The model agrees
  (T2): the M4 ≤ 0 share at P376 goes from 0.45 to 0.26, and the
  worst-case rate rises 4.0x (P376) and 4.1x (P500). The fixed-schedule
  factor at P500 falls from 50/287 to 15.5/88.6 (3.2x). The prior "4.1x at
  P376" was a pass-count ratio. It matches the worst-case rate ratio here;
  the mean C ratio is 2.95x.
- **Bytes (stage 5).** **e changes bytes.** pk and sm differ from the pin
  for every seed, because keygen also runs the norm equation and the draw
  sequence changes. Stage 5's replay of PRISM_v2 under a seed therefore
  breaks. That needs a reference-replay mode (the pin's e-2 search behind a
  test flag) or a re-pinned reference built with used = e. Keep three
  assertions apart: same ideal / signature accepted / identical bytes. The
  key and signature formats (lengths, encodings) do not change.
- **The postcondition is load-bearing.** With it disabled, about 97% of
  runs abort at `assert(ibz_is_odd(d1))` in `dim2id2iso.c`. The theta
  postcondition is what delivers odd d1 to Kani in this implementation;
  the Sage Qlapoti instead handles the other theta classes with diagonal
  "product" steps (`fix_end`). So O29 and the postcondition are separate
  questions. The postcondition rejects about 78% of Cornacchia successes
  on the reference structure (T2), at every exponent.

**Verdict:** e is correct and safe with PRISM_v2's chain (square-root
variant, no extra torsion). It is 2.3–3x fewer passes on the reference and
about 3.2x on a fixed schedule at P500. It changes keygen and signature
bytes, so it is a replay-breaking proposal for a design round, not a
silent fix.

## Sub-experiment 3: survey and Qlapoty-style prototype (T5)

The ePrint PDFs were behind Cloudflare. The survey uses abstracts, search
excerpts, the SQIsign 3.0 spec in the repo, the Qlapoti GitHub sources and
docs.rs; check citations against the papers before relying on them.

| construction | removes S-dependence? | expected trials | fixed schedule | fit for PRISM |
|---|---|---|---|---|
| Qlapoti (2025/1604), = PRISM_v2 | no (area law in n/sqrt p; reference iteration bounds c = 2185 / 38495 / 21484 for the PRISM primes) | 1/rho, rho down to ~5e-5 at P376 e-2 | yes with padding (sub-exp 1) | current |
| Qlapoty (2026/1700, DLS26; SQIsign 3.0) | partly: a balanced alpha0 (Loop1) removes the generator dependence; rho still depends on n via 2^e/n² | the paper reports 6–9x faster than Qlapoti | Loop1 + LoopTwo over odd lambda; no extended Cornacchia, no postcondition in SQIsign (its chain has extra torsion) | promising, prototyped |
| ct-Qlapoti (2025/2192, Alg. 7) | n/a | naive fixed bound from Qlapoti's table | yes, about 9 s at level I in the proof of concept | confirms the padding route is expensive without restructuring |
| Leroux–Schaeffler (2026/2153) | inert ideals of every norm, integer bounds O(p^4) | - | designed for fixed-size integers | applies to PRISM; relevant for the integer-width channel, not prototyped |
| sqisign-rs `quat/qlapoty.rs` | Rust Qlapoty with a constant-time 2-D reduction | - | partial | implementation reference |

**Prototype, P500 (odd n, 49.5% of S), per odd lambda at e:** z ≤ 0 in 7%
(42% at e-2). With PRISM_v2's postcondition, 50% of solutions are rejected
(77% on the reference structure). There are no dead alpha0 at e. Fixed
schedule (best of one- and two-stage), padding factor at 99.9% / 2^-64:

| P500 variant | e-2 | e |
|---|---|---|
| p: z prime, postcondition kept (works with the current chain) | 72.5 / 415 | **6.2 / 35.2** |
| pe: + extended Cornacchia | 35.0 / 200 | 4.3 / 24.8 |
| pn / pen: no postcondition (needs a chain that takes every theta) | 30.9 / 177 · 16.5 / 94 | 3.4 / 20.0 · 2.4 / 13.9 |

Limitations:
- The prototype handles odd n only.
- Its Loop1 is simplified. 2% of S never find a balanced alpha0 within
  2000 iterations (T5 "stuck"); they are left out of the sizing.
- The "p" cell keeps PRISM_v2's postcondition test. Whether Qlapoty's
  outputs need it (odd d1 by construction) is not settled.
- None of this changes the secret-key format. It changes how the response
  ideal is found, so key and signature bytes change.

## Recommended design (proposal)

1. **Target exponent e** (O29), postcondition kept. C-validated at all
   three levels. Design round: replay mode for stage 5.
2. **Reference structure, fixed schedule, two-stage**, which handles every n:
   - a generator pool (D draws, oblivious compaction into B generators);
   - blocks of M = 64 public prime lambdas per generator;
   - a cheap stage per slot: lambda^-1 mod n, target, nearest plane, M4
     sign, residue condition, sieve by non-good primes < 2^12 (85 words);
   - oblivious compaction of survivors into Q expensive slots: good-prime
     stripping, a fixed-length primality test, Cornacchia with T_corn
     steps, the postcondition as a mask;
   - output the first success by a masked select.
   - Caps: T_lat = ceil(0.3932 nb)+3, T_corn = ceil(0.7202 mb)+3, no skip
     loop, n_min = 2^(nb-34).
3. **Next step, gated on odd-n handling:** Qlapoty-style search at e,
   which is about 2.5x cheaper again at P500 (6.2 / 35.2).

Fixed-schedule cost, model estimates, sized for 2^-64 (99.9% in
brackets). Work in millions of units; factor against the reference mean
at e-2:

| set | prior fresh (e-2) | recommended: two-stage at e (D, B, P, Q) | work (M units) | Qlapoty-style p at e (odd n only) |
|---|---|---|---|---|
| **P500** | 847 (132) | **88.6 (15.5)**; D 28,538, B 833, P 53,312, Q 12,400 | 112 (19.5) | 35.2 (6.2) |
| P248 | 317 (49) | 64.5 (11.1); M 32, P 12,544, Q 3,229 | 20.2 (3.5) | 40.0 (6.9) |
| P324 | 249 (39) | 57.5 (10.0); M 32, P 16,288, Q 4,152 | 29.3 (5.1) | 33.9 (5.9) |
| P376 | 2513 (391) | 198 (34.6); M 64, P 91,392, Q 18,994 | 145 (25.4) | 120 (21.0) |
| P664 | 730 (114) | 65.4 (11.4); M 32, P 51,648, Q 12,325 | 139 (24.2) | 36.1 (6.3) |

The fixed schedule must run the 2^-64 column; the 99.9% column is for
comparison only. At P500 the recommendation is 9.6x below the prior
fixed-schedule estimate. It is still about 89x the variable-time mean.

## Remaining secret-dependent channels

1. **n < n_min.** With n_min 34 bits under the bound, an S with a smaller n
   makes every pass fail. Its probability (~2^-64 by the quadratic tail of u)
   is part of the failure budget; it needs a proof, not the fit.
2. **Generator validity.** The worst-case bound (0.039 per draw at P500)
   is empirical over 600 S. Adversarial n with many small prime factors
   could fall below it, and the pool then fails more often. A proven lower
   bound, or a check on n, is needed.
3. **Worst-u extrapolation.** The success rate at u = 1/sqrt 2 comes from
   a power-law fit (scale 0.80 at P500, e). The 2^-64 tails assume blocks
   are independent beyond the measured per-generator mixture.
4. **Transformed case.** Endtype 1/2 runs the chain at used-1. The chain
   length then depends on the secret, so stage 4 must mask it (run e steps
   with a dummy step, or always run e-1 and pay 2x on rho).
5. **Oblivious primitives.** The design assumes constant-time versions of
   gcd and inverse mod n, the primality test, Cornacchia (fixed T_corn),
   the sieve and the compaction network (2 P log P), with fixed-width
   integers (the Leroux–Schaeffler bounds are relevant).
6. **The postcondition** is a mask on the output; the theta class must not
   steer control flow.
7. **The Qlapoty route** adds n parity (even n is about 50% of S), the
   Loop1 cap and the prototype's stuck ideals.
8. **Not covered:** keygen's prime-norm search, and the chain itself
   (stage 4).

## Open questions for Rick

1. **O29.** Adopt e (C-validated, about 3x cheaper fixed schedule at P500)
   at the cost of stage 5 byte replay? Options: a pin-replay test mode with
   e-2 behind a flag, or re-pin the C reference with used = e.
2. **The postcondition.** Keep PRISM_v2's square-root chain and the theta
   postcondition, or move to a SQIsign-3.0-style chain (extra torsion,
   e-2)? That costs about 3x in rate and needs evidence that it accepts
   every theta. Or implement the Sage-style diagonal steps? That doubles
   the rate on the Qlapoty route (pn) and quadruples it on the reference
   structure.
3. **Even n for Qlapoty.** Restrict S to odd norm (choose an odd-norm
   equivalent ideal, which raises the u bound slightly), or translate the
   odd part plus masked 2-isogeny steps (this touches stage 4)? Either is a
   design round.
4. Is 2^-64 the right failure target for the fixed schedule? The cost
   scales with log(1/eps): 2^-64 is 5.7x the 99.9% work.

None of these change the secret-key format; all change bytes.
