# Notebook

Entries before 2026-11-13 are a backfill, written on 2026-11-13 from the commit history, the pull requests, the review documents and the recorded rulings. Where the backfill could not establish a detail it says "not recorded". Entries from 2026-11-13 on are written with the work.

"Reviewer" is the AI design partner, "agent" is whichever AI coding agent was working in the repository at the time (more than one tool was used; see `CONTRIBUTIONS.md`), and Rick is the lead architect. Dates are the local commit dates of the development machine (UTC−5). Commits are on `dev` unless a branch is named.

---

### 2026-11-07 — Repository started; the carried index
Question: How does a verifier of the §5 (strongly unforgeable) variant find the challenge degree without repeating the signer's hash-to-prime search?
Done: initial tree `8641732`; `44ec65b` carries a 16-bit index `j` and tests one degree.
Result: Ruled by Rick: the signature carries `j`; the verifier derives one scalar and tests one `q_j`. Reason: constant verifier cost.
Who: Rick ruled / agent implemented / reviewer reviewed (PR #1; `0ebbb56` resolves the five review findings).
Corrections: 2026-11-12, the index was replaced by a salt (see that entry).

### 2026-11-08 — PR #1 and #2: verifier skeleton and chain port
Question: Can Algorithm 3 be built from the C reference's theta code?
Done: PR #1 (`d370f79`) dropped basis hints (the public key is the Montgomery coefficient alone) and refuses the starting curve `A = 0`. PR #2 (`f49b08d`) ported the (2,2) step, splitting and the gluing chain. PR #3 (`690ad8e`) added CI.
Result: Ran: chain known-answer tests against the C tree. The accept decision was not yet sound.
Process note: commit `8f1f8bb`, pushed to `dev` on 11-07 before PR #1 was opened, carries a Cursor co-author trailer. It is the only commit on `dev` that does; no commit of PR #1 carries one. The CI hygiene job checks for such trailers since.
Rulings (Rick, 11-08): the Raspberry Pi 4 floor-device run is manual and its result is committed with the SHA; PRISM is not near shekyl-core; the basis search is bounded at `2λ` Legendre symbols.
Who: agent implemented / reviewer reviewed.

### 2026-11-09 — The identification problem; `e′ = e − 2`
Question: Algorithm 3 reaches the midpoint surface twice, in two theta structures. How are they identified?
Expected: derivable from the rational `2^e`-torsion.
Result: Derived, from the literature (Dartois 2024/1180 Rem. 18 and App. C.2; SQIsignHD App. C.2): it cannot be done when the chain length equals the torsion exponent, because the last two steps need square roots with free signs. The paper does not address it. `955b723`, `c5ade08`.
Conclusion: Ruled by Rick: the two extra bits come from the chain, not the field. `e′ = e − 2` (246/374/498), `q = 2^{2e′−1} − s²`; the paper's primes are kept (`3af52c7`, PR #4). A larger prime would have cost a limb at levels I and III. The verifier returns `Unidentified` and never accepts until identification is built.
Also: PR #6 (`d17b520`) recorded one §5 signature per level from the C tree's functions. The operation-count metric was approved as a metric, not a gate.
Who: reviewer proposed / Rick ruled / agent implemented.
Corrections: 2026-11-10, the PR #6 vectors were found to have points of the wrong order and a wrong basis export (below).

### 2026-11-09 — Email to the PRISM authors
Done: Rick wrote to three of the authors asking whether a `2^e`-only identification exists and whether §5 intended a larger prime, with three smaller points.
Result: no reply as of 2026-11-13.

### 2026-11-10 — Small-extension torsion recheck; dual-structure plan
Question: Does the new `q` window at `e′` reopen the Nakagawa–Onuki small-extension attack?
Result: Ran (PR #9, `553c881`): cleared. `√q` alone exceeds `2^λ`; orders of `−p` mod `q` all above `2^24` over 3,000 samples.
Done: PR #10 (`4903adf`) wrote the dual-structure conditions into the spec before any code.
Who: agent / reviewer reviewed.

### 2026-11-10 — The signature had lost two bits of torsion
Question: Does the planned dual-structure construction have the inputs the published code (Theta_dim4 `KaniEndoHalf`) expects?
Result: Ran (reading `export_s5_vectors.c`): the exporter doubled the image basis twice, so signature points had order `2^{e′}`. The published construction consumes the image at order `2^{len+2}`. Algorithm 2 as printed emits order-`2^e` points; the loss came from substituting `e′` for `e` everywhere.
Conclusion: Ruled by Rick: keep the full-order points, chain stays at `e′`. "Be robust rather than edge running."
Who: reviewer found / Rick ruled.

### 2026-11-10 — Step 2 brief; three reviewer errors recorded
Done: one consolidated brief for the accept path, with each claim marked Ran, Derived or Open.
Reviewer errors, found later:
- Claimed the canonical second basis point is the one with `x(Q) = −x(P) − A`. That point is `P − Q`. A probe "ran the whole chain" with it, which was read as confirmation. A completed chain proves only that the kernel is isotropic.
- Told the agent to validate vectors by the Weil pairing identity. A pairing pins the determinant and nothing more (next entry).
- Reported a level V "sign mismatch" as a separate defect. It was the same export fault.

### 2026-11-10 — The model refutes, then confirms, the construction
Question: Do the two half-chains meet (`C1.zero() == C2.hadamard().zero()`) on real §5 signatures?
Expected: yes.
Result, first run: Ran, refuted at level I (`fe016d0`).
Diagnosis (agent): three layers. (1) The model re-completed `M1`; in the reference, `M1` is glued from the complement of `M1_0` and `−Φ̂` of the complement of `M2`, and its symplecticity is a check. (2) After the fix the refutation survived, while Dartois's dimension-4 code and 72 small dimension-2 instances passed. (3) An instrumented exporter showed the signing pipeline correct and the verification-key export wrong: it emitted `(−P, Q − P)`, a determinant −1 shear that every pairing check accepts and the chains reject.
Result, after `177eae5` and `8351588`: Ran, confirmed at level I on two keys; `91a03a6` confirms levels III and V.
Conclusion: the dual-structure construction works on real §5 signatures at all three levels. Pairing-level validation of vectors is insufficient.
Who: agent diagnosed and fixed / reviewer read the diffs.
Corrections: 2026-11-13, raised in review of PR #17. The identity `e(R₁, R₂) = e(P₁, P₂)^q` holds for determinant +1 images of the preimage pair only; a determinant −1 image inverts the pairing, and the verifier's tests refuse a swap of the two points at the pairing check. The limit of a pairing check stands on the determinant +1 cases of model negative row 14. Open: this entry, the model `REPORT.md` and `177eae5` describe the faulty export as a determinant −1 shear that the pairing checks accepted. Both cannot hold for the identity as now checked; which of the two descriptions is wrong (the determinant, given the free lift sign, or what the model's checks of that date compared) is not established.

### 2026-11-11 — Negatives, basis rule, pairing check
Done and Ran:
- Negative table at three levels (`91a03a6`, corrected in `fa0673b` after the reviewer found the model normalized the sign of `P` and `Q` independently).
- `(P, −Q)` runs both chains to completion and is refused only at the codomain comparison. A non-isotropic input does not stop a chain. An explicit Weil pairing check was added before the chains (`ed62e26`), giving up the pairing-free verifier.
- Basis rule: over 101 keys per level the C `Q` is always the difference of the cofactor-cleared points; the other variant matches about half the time (`c357adf`).
- With the canonical-model check off, other Montgomery models of `E_sig` are accepted. That check is what prevents a second valid encoding.
- Order-4 and order-2 translates of the signature points: none accepted.
Reviewer errors: claimed some order-4 translates reach the image check; with joint normalization and the pairing check none do (the `[2^{e−1}+1]` scaling is the case that does). Gave the collision bound as `2^{122.5}`; the challenge is odd, so it is `2^{122}` before the primality filter.

### 2026-11-11 — Fault in the pinned C reference
Question: Why does key generation abort on 24 of 24 seeds at level III?
Result: Ran (agent, backtrace): `ibz_cornacchia_prime` is handed a negative cofactor by `quat_qlapoti`; its loop test stays true for a zero remainder and the next iteration divides by zero. The primality test upstream looks at the absolute value. At level V key generation aborts on 14 of 24 seeds and signing on a further 8, so 22 of 24 give no signature; at level I the counts are 2 and 5 of 24. Every abort is the same division by zero (model `REPORT.md`, "Exporter aborts").
Done: `887ea47` rejects a non-positive cofactor in the exporter build only; the pin is untouched.
Open: report upstream. The level III accept vectors generated before the patch are the seeds on which key generation happened to return.
Who: agent found / reviewer read the routine.

### 2026-11-11 — Closed-form symplectic matrices
Question: The Rust completion of `M1`, `M2` by a linear solver gave a different valid representative from PARI's, so the model's null points could not check the Rust chains. Which representative should the verifier use?
Result: Derived: because `q ≡ −s² (mod 2^e)`, both matrices are explicit in `s` and `t = s⁻¹`. No solver is needed. The reviewer reports a check over 6,000 random odd `s` across three levels; its output is not in the tree, a commit message or a pull request, so under rule 5 it is not Ran. Ran, in the tree: with the closed form, chain verdicts match the library completion on the 73 honest signatures then committed and on every negative row, and the theta base change matches the committed matrix files (`bf20794`).
Done: `bf20794`; the solver was deleted.
Who: reviewer proposed / agent implemented.

### 2026-11-12 — First accepted signatures; Raspberry Pi 4
Result: Ran (`49691c4`, PR #13 `d018757`): `verify` returns `Ok(())` on every committed honest signature at all three levels, and each negative refuses as recorded. Per-step theta null points match Sage on the six original vectors.
Result: Ran (`60115a1`, manual run on the Pi 4): about 167, 449 and 1023 ms per verification at levels I, III, V, unoptimized. Record in `docs/benchmarks/`.
Process note: the last commits of PR #13 were merged before the reviewer had read them; they were read on 2026-11-12 and matched the design.

### 2026-11-12 — Security argument for the unsalted protocol (PR #14)
Done: `SECURITY_ARGUMENT.md` with obligations A1 (polarized map), A2 (primality), A3 (encoding uniqueness) and a conditional decomposition. `e6f90bc`.
Result: Ran (reviewer, then agent; 1,500 first-prime searches per level in the committed measurement): prime density 1.089%, 0.719% and 0.526% at levels I, III, V, giving a collision-replay cost near `2^125`, `2^190`, `2^252` hash queries, below every row's nominal level. The paper concedes about `2^127` for its own level I parameters.

### 2026-11-12 — The salt
Question: Should the public index be replaced by a salt?
Expected (reviewer): yes, for three reasons: collision replay, the index revealing skipped candidates, and messages that can never be signed.
Conclusion: Ruled by Rick: yes, and publish the comparison with and without. Hedged derivation (seed, message, caller randomness) adopted after Rick asked whether the listed weaknesses came from deriving the salt.
Reviewer error: called the salt a fourth departure from the paper. The paper states that its salt proof applies to a salted version of the §5 variant.

### 2026-11-13 — Comparison results (PR #15, branch `protocol/salt`)
Done: merged as `6b93ecb` (branch head `cb9f6ec`, archived at tag `archive/protocol/salt-2026-11-13`). The E1, E3, E5 and single-key E6 records were taken at `9d2aef5`, the twelve-key E6 records at `e5635d0`; each record's `.provenance` names its commit and command.
Result: Ran. Records under `docs/comparison/`; `scripts/analyze_comparison.py` regenerates every table.
- Collision replay at reduced challenge width, real signer and verifier: 100 of 100 accepted unsalted, 0 of 100 under the salt; fitted cost slopes 0.489 and 0.962 against predicted 0.5 and 1.0.
- Index leak: not observed. All 113 archived signatures sit at the first prime index.
- Unsignable message: not found in 26,000 messages on one key (20,000, 4,000 and 2,000 at levels I, III, V), nor in 24,000 over twelve further keys at level I.
- Verification time unchanged within noise.
Conclusion: the case for the salt rests on collision replay. Two of the reviewer's three reasons were structural possibilities this backend never exercised.
Rulings (Rick, 11-13): AI participation is disclosed in full; the paper waits for a fully viable signature system and a usable repository.
Who: reviewer wrote the brief and reviewed at `07ec463` / coding agents implemented and ran (Codex CLI to `9972661`, which includes the implementation checkpoint `6caebb6`; Claude Code for the drivers, experiments, records, report and review fixes, the branch's commits from `b98925f` to `3770c82`) / Rick ruled, added `bf163f3` and `cb9f6ec`, and merged.

### 2026-11-13 — Lab notes started; the backfill checked against the tree
Question: Do the hashes, counts and figures of the backfilled notebook, ledger and contributions record match the repository?
Expected: mostly; the backfill was written from memory of the history by the reviewer, which has no access to run anything.
Done: at `dev` `6b93ecb`. Every cited hash resolved with `git cat-file` and its subject and pull request compared with the entry citing it; every count and figure searched for in the tree, the commit messages and the pull-request descriptions.
Result: Ran. All 25 cited hashes exist on `dev`. Six statements were wrong or unsupported and are corrected in place, because this is the pull request that first commits them (rule 3 applies from here on):
- `8f1f8bb` was cited as a review. It is the second commit of the repository, and the review of the carried index is PR #1.
- The AI attribution trailer was placed on PR #1. It is on `8f1f8bb`, pushed before PR #1 existed.
- Level V "aborts on 22 of 24" was given as a key-generation count. Key generation aborts on 14 of 24; 22 of 24 is key generation and signing together.
- The 6,000-sample check of the closed-form matrices was marked Ran. No output of it is committed. The claim is Derived, with the committed evidence named.
- The coding agent was described as Claude Code throughout. The machine's records show three tools (`CONTRIBUTIONS.md`).
- "Prime density near 1%" for the PR #14 measurement is replaced by the measured figures.
Two more were raised by the automated review of this pull request and are corrected the same way: ledger C6 said the pairing identity holds for determinant ±1 images (it holds for +1; see the correction under the 2026-11-10 entry), and ledger C11 said signatures came "at the first candidate" where the records show the first candidate of prime degree.
Not checkable from the repository, left as written: the rulings and their dates, the reviewer's own errors, the email to the authors, and the paper's level I figure of about `2^127` (the PDF's parameter table shows 127 at level I; the sentence was not traced further).
Open: `SECURITY_REVIEW.md` still says no notification to the paper's authors has been sent, against the 2026-11-09 entry above.
Conclusion: from this entry on, every pull request adds its own entry here and updates the ledger rows it touches, in the same pull request.
Who: Rick asked for the check / coding agent (Claude Code) ran it and made the corrections / not yet reviewed.

### 2026-11-13 — Verifier performance (PR `perf/verifier-ops`): the instrument first
Question: Where does a verification spend its field operations, and how much of that can be removed without changing what is accepted or refused?
Expected (reviewer, Derived, in the brief): removing the per-step inversions and the duplicate scalar multiplications cuts level I by 30 to 45% in instructions, and squaring adds to that. The reviewer's callgrind run at `6b93ecb` (its Ran, on another toolchain, not reproduced here) puts 25% of level I in extension-field inversion and 29% in Jacobian scalar multiplication, and estimates about 1,960 inversions in the chains and 1,980 in the pairings.
Expected (coding agent, before counting): inversions near 4,000 at level I if that estimate is right; the challenge stage near zero, since neither the hash nor Baillie–PSW uses the field; no refusal dearer than acceptance, since every refusal stops early.
Done (`c9df786`; command `sh scripts/op_counts.sh`): an operation counter in `prism-field` (feature `op-count`, enabled by no manifest), ten stage marks in the verifier, one generator for the negatives shared by the acceptance tests and the reporter, and `scripts/op_counts.sh`, which writes `docs/benchmarks/op-counts.txt` and `negative-verdicts.tsv`. No optimization in this commit. The stage marks moved no computation: the two chain kernels are still built directly before their chains.
Result: Ran. Level I primary vector: 345,200 extension-field multiplications and 4,019 inversions, of which 1,988 are in the pairing stage and 2,012 in the two chains. Levels III and V: 530,434 and 6,072; 722,924 and 8,056. The challenge stage counts zero. The dearest refusal of every primary vector is `Image`, at about 90% of the accepted cost. 2,712 negatives over the six primary vectors give eleven of the twelve refusals; `Basis` has no vector.
Squaring. Expected (reviewer, Derived): `fp2`'s extension-field square costs two base multiplications against three, at 79 sites of the form `x.mul(x)`; its note that base-field squaring loses at 23 limbs or more does not apply at 4 to 8 limbs. Expected (coding agent): squaring faster at every level on x86; the verdict of every negative unchanged.
Done (`1334dc2`): `Field::square`, 79 sites converted, and products the Jacobian doubling and addition computed two or three times computed once. An equality test over the named elements and 100,000 stream elements per level and field, and a fuzz target for the same comparison.
Result: Ran. Level I: 184,098 multiplications and 135,822 squarings where there were 345,200 multiplications; the 25,280 that vanished are the repeated products. All 2,712 verdicts are unchanged. On the development machine a squaring takes 30.6 / 55.4 / 99.5 ns against 43.2 / 86.1 / 152.7 for a multiplication, so squaring is kept at every level. The arm64 measurement is the Pi run.
Tried and dropped: `use_sum_of_products = true`. Faster at level I (38.2 ns a multiplication) and slower at III and V (97.2 and 164.4), so not adopted. Record: `docs/benchmarks/sum_of_products_x86_64_2026-11-13.txt`.
Error (coding agent), found before any commit: the first conversion was a text substitution whose pattern also matched `x.mul(x)` inside `point.x.mul(x)`, turning six products of two different values into squares. The compiler showed it as four unused parameters in `theta::scale_with`. The substitution was undone and redone with a pattern that cannot start inside a longer receiver, which converts exactly 79 sites, and the verdict table and all 113 honest vectors were rerun. A count of operations would not have caught this: it turns a multiplication into a squaring either way.
Error (coding agent), found by CI on the first push: the new `op-counts` job was added to the workflow but not to the list `aggregate.py` holds the workflow to, so `hygiene` failed. Fixed in `e22ea4e`.
The pairing. Expected (reviewer, Derived): the affine Miller loop inverts twice a step, about 1,980 inversions at level I; carrying the Miller value as a fraction and the running point in Jacobian coordinates leaves a constant number. The risk named in the brief is the refusals: a zero factor carried to the end looks like one late failure or like a value. Expected (coding agent): the same field element, since the fraction is the same product; the same refusals if each factor is tested where it arises.
Done (`c9ff6ba`): `pairing::miller` keeps `T` in Jacobian coordinates, returns numerator and denominator, and tests the tangent value and the vertical value for zero at each step; `weil` inverts three times whatever the order. The affine loop is kept as `pairing::oracle`, compiled for the crate's tests and for the fuzz crate only.
Result: Ran. Level I inversions 4,019 to 2,033 (pairing stage 1,988 to 8), for 10,896 more multiplications and squarings. All 2,712 verdicts unchanged; the pinned Sage pairing values pass unchanged. Against the affine loop: the same value on 1,000 full-order pairs per level and the same value or refusal on 250 unconstrained pairs per level (`a_thousand_random_pairs_per_level_agree`, release, 42 s); the same on 50 adversarial pairs per vector (equal, negated and dependent points, each point doubled down to order two and to infinity, 2-torsion translates, off-curve points, the point at infinity), each also at one doubling too few and one too many. Fuzzing: `field_square` 3,974,415 executions in 601 s with no failure; `weil_matches_affine` 10,484 executions in 601 s with no failure, each execution four scalar multiplications and both pairings under the address sanitizer.
Refusals, how they were kept: the affine loop refuses on a zero tangent value, a zero vertical value, the running point reaching order two before the last step, and not reaching it at the last step. With `Z ≠ 0`, which holds from the on-curve check on and is preserved by a doubling with `Y ≠ 0`, each of those is the vanishing of one numerator the new loop already computes (`2Y(yZ³ − Y) − M(xZ² − X)`, `xZ₃² − X₃`, `Y`), so each is tested at its own step and nothing is inferred from the final fraction. The final inversion cannot fail: its argument is a product of factors already found nonzero.
The chains. Expected (reviewer, Derived): `invert_coords` inverts four coordinates at every chain step, about 1,960 inversions at level I; Montgomery's trick gives the same four elements for one inversion, and a whole chain's inversions may be batched if every outcome is unchanged. The risk named: a zero theta coordinate must still refuse. Expected (coding agent): about one inversion per chain plus the gluing's constant; the verdict table unchanged.
Done (`1705c98`): `invert_all` (one inversion for any number of elements, with an explicit zero test on each element) and one call per chain for the inverse coordinates of every null point, after the steps. The gluing's own four use the same routine. The cofactor form of the brief was not used, so the inverses are the same field elements and nothing downstream changed. The 4 × 4 `inverse` and the gluing's other inversions are left: 26 a chain, independent of its length.
Result: Ran. Level I inversions 2,033 to 73 (78 at III and V), for 5,892 more multiplications. All 2,712 verdicts unchanged. Tests: the shared inversion equals each element's own inverse at six lengths; a zero in every position of three lengths is refused; a zero in each coordinate of each of three theta points is refused.
Refusals, how they were kept: a zero coordinate of the null point at step `i` used to stop the constructor at step `i`; it now stops it after the last step. Every failure inside `DualChain::new` is the same `None`, which the caller turns into `Kernel`, and no other check runs between the two places, so no refusal moved past another. The chain's remaining steps run on a null point with a zero coordinate before the refusal; they are total functions and their results are discarded. The cost of such a refusal is therefore that of the whole chain, which is still below acceptance.
Scalar multiplications. Expected (reviewer, Derived): one verification runs 27 full-width ladders; `[s]P_sig` and `[s]Q_sig` are computed twice, two `dbl_iter(…, e − 2)` twice, many coefficients of `M1` and `M2` are 0 or ±1, and pairs of ladders could share doublings. Expected (coding agent): the sixteen kernel ladders run on four base points, so one table of doublings per base point serves all of them and the two image ladders as well; doublings fall from about 27e to about 7e, near 18% of level I's products.
Done (`aa6048b`): the key's basis is lifted and the signature points scaled once, in the pairing check, and handed to the chains. A table `[2^k]X` per kernel basis point; every ladder on that point becomes additions from the table in the order the old ladder made them. The two `dbl_iter(…, e − 2)` are table entries. `[s]` of the image points on the response's curve is the scaled pair already computed, and `[s]` of each difference is the difference of the two. A coefficient equal to `2^e − 1` is taken as a negation.
Result: Ran. Level I: 231,787 products where there were 336,708 (137,836 multiplications and 93,951 squarings), 31% fewer, more than expected because the image ladders on the response's curve disappeared entirely. Inversions 71, square roots 6. All 2,712 verdicts unchanged. Before the `−1` case the new kernel had the same coordinates as the old on all 113 vectors and 36 random sets; with it, the same points.
Measured before deciding the `−1` case: of the 16 coefficients of a verification's two kernels, over all 113 vectors, 8 are zero, 1 is one, 2 are `−1` and 5 are neither. A zero costs nothing from a table and a one costs one addition. The two `−1` cost `e` additions each, 4.7% of level I, so they were worth a case of their own. They are recognized by value, `2^e − 1` exactly.
Refusals, how they were kept: the refusal order is unchanged because no check moved. The lift and the scaling happen where the pairing check already did them; a basis that does not lift is still `Pairing`; no chain work runs before the pairing has passed. Taking `2^e − 1` as `−1` needs each basis point to have order dividing `2^e`; the order checks that precede the pairing establish that for the key's basis and the signature points, and the other two basis points are multiples of those. No refusal costs more than acceptance: the dearest is `Image` at 97%.
Candidates seen and not built: the order and span checks (the `torsion` stage, 18,000 products at level I) repeat doublings that the tables later make again in another coordinate system; the 26 inversions of each gluing could share one; the scaling of the two signature points and the multiplication by `q⁻¹` are three ladders that a window method would shorten; the Miller loop and the table doubling of the same point walk the same doublings twice.
Overall. Result: Ran. One accepted level I signature costs 137,836 multiplications, 93,951 squarings and 71 inversions where it cost 345,200 multiplications and 4,019 inversions. On the development machine, five alternating runs in one session at load near 4: 24.1 to 11.5 ms at level I, 68.4 to 32.5 at III, 156.4 to 73.9 at V (`docs/benchmarks/verifier_ops_x86_64_2026-11-13.txt`). The reviewer's estimate was 30 to 45% of instructions from the inversions and duplicate ladders, with squaring on top; the measured fall in time is 52 to 53%.
Against the brief's acceptance list: all 113 honest vectors accept in debug and release (the brief said 67; the tree has 113); every one of the 2,712 negatives has the verdict it had before the first optimization, held by `negative-verdicts.tsv` in CI on x86_64 and aarch64; the Sage-pinned pairing values and null points pass unchanged; the count records were regenerated in every commit; no refusal costs more than acceptance at any commit; no new dependency, no `unsafe`, `fp2` at `=0.6.0`. The fuzz corpus in the repository's sense is the 113 honest signatures the seeder writes, which are the accept vectors; no larger saved corpus existed to replay.
Error (coding agent), found by an independent review of the finished branch: `BENCHMARKING.md` said no base-field call is made directly at any stage. The basis search calls the base field's residuosity test, 0 to 32 times a verification; the displays the claim was read from had the base columns cut off. Corrected. The same review found that the comparison "no refusal dearer than acceptance" had only been made against each negative's own vector, at a 3% margin; per level, the dearest negative is still below the cheapest honest vector (226,185 against 230,605 products at level I), and that is what the document now states.
Open: the Pi 4 runs at `c9df786` and at the head, and the cycles per operation derived from them.
Who: reviewer wrote the brief / coding agent (Claude Code) implemented and ran / not yet reviewed.
