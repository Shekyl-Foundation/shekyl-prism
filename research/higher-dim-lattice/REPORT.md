# Raising dimension, and a one-characteristic ablation, against 4-D H2

Date: 2026-11-22. Research for Shekyl PRISM. Step-counting model, no
constant-time or security claim. This is not a proposal to change the
signature format.

The 4-D experiment found that a *fresh* hash-chosen ideal (H2) flattened
the 2-D-style target dependence, while a unimodular mask of a fixed 4-D
ideal (H1) kept 21–26% of the trace variance, and that LLL still costs
about 1.5 steps per bit of the challenge. A later attempt to *pad* the
same 4-D grid to P664 volume moved the cost, but did not make LLL
size-blind. This report asks whether *raising the dimension*, rather
than the volume, changes that law — and, in the same harness, which
lattice characteristic actually buys target-independence and a low-CV
trace.

The study is a one-characteristic-at-a-time ablation against the 4-D H2
baseline. Dimension and fresh-lattice selection are varied separately,
so the 2-D→4-D jump is not treated as a single effect.

**Coverage.** All five Shekyl sets (P248, P324, P376, P500, P664). 24
keys × 10 messages per set (240 stream rows and 240 product rows per
arm per set; 1,200 stream rows per arm across sets). Salt is
SHAKE-256("salt" ‖ sk ‖ msg) of length 32–96 bytes, and
q = FS(pk, salt, msg), as in the scheme. A 180-check style membership
test is the per-row `ok_q` / `ok_in` flag (q divides nrd of a short
vector that lies in the ideal, or in a 4-D block of it).

Within a set, bits(q) does not vary (q = 2^{2e−5} − s² always has
length 2e−5). The column "η² vs size" below is therefore η² of the
step count against the parameter set, i.e. against challenge *size*,
not a fixed-size target η². Fixed-size target η² was measured in the
4-D experiment (H2 at chance, H1 at 21–26%). η² vs the secret key is
measured here on each arm's product panel (chance is about 0.10 for
24 keys × 10 messages).

## 1. What each arm changes

The baseline H2_4 reduces a hash-chosen left ideal of norm q, from its
HNF basis, with Cohen's integral LLL at δ = 99/100, under the
reduced-norm form, in exact integer arithmetic.

- **Selection (D0_4, H1_4).** D0 is the deterministic smallest-y ideal
  of the same q. H1 is a 128-bit unimodular mask of that D0 lattice:
  the same 4-D geometry as D0, not a fresh ideal. This is the
  historical "dimension alone did not flatten the target" control.
- **Dimension (EMB6, EMB8, EMB12, EMB16, DS8).** EMB* take the *same*
  H2 ideal as the baseline and extend it to rank n by salt-chosen
  extra generators inside the ideal, plus extra axes of *fixed* scale
  2^{16}, independent of q. A short vector of the high-dimensional
  lattice projects onto an element of I_q (the first few reduced
  vectors may have a zero projection, because the extra axes are
  short; we search the whole reduced basis). DS8 is the direct sum of
  two H2 ideals of the same q (a second class from a salt-shifted
  seed): a block embedding in O ⊕ O, not a Kani correspondence. No
  Kani or higher-dimensional isogeny paper was re-read for this
  experiment, so none is cited.
- **Covolume (SIZE_COF).** A salt-chosen cofactor so that r·q is about
  2^{1323}; the lattice is still rank 4. Internal only.
- **Starting skew (SKEW_U).** The same H2 lattice as the baseline, with
  a 128-bit unimodular applied to the HNF start. Only the presentation
  changes.
- **Form (FORM_EUCL).** Same lattice, LLL under the Euclidean form
  instead of the reduced norm; the unimodular is applied to the exact
  basis and checked under nrd.
- **Precision (PREC_256).** Same lattice and form; the Gram is
  quantized to 256-bit entries.
- **Delta (DELTA_75).** Same lattice, form, and start; δ = 3/4 instead
  of 99/100.
- **Ignore (QARY8, FIX8).** 8-D q-ary lattices that do not encode the
  norm equation. QARY8 has modulus q, so its volume still tracks the
  challenge. FIX8 has a fixed modulus 3329, so its volume does not.

## 2. Summary table (stream, 1,200 rows per arm)

Slope is steps per bit of q across P248–P664. Means and CV are at
P248 and P664. Cost is versus H2_4 at the same set. "Solves" is the
fraction of rows with a short vector that lies in the ideal and
satisfies q | nrd.

| Arm | Changed | Dim | Solves | Slope (R²) | η² vs size | η² vs key | Mean P248 → P664 | CV P248, P664 | Cost P248 |
|---|---|---|---|---|---|---|---|---|---|
| H2_4 | — | 4 | 100% | **1.509 (0.992)** | 0.992 | 0.11 | 741 → 1997 | 0.039, 0.024 | 1.00× |
| D0_4 | selection | 4 | 100% | 0.455 (0.980) | 0.980 | 0.09 | 231 → 610 | 0.063, 0.038 | 0.31× |
| H1_4 | selection | 4 | 100% | 1.564 (0.557) | 0.559 | 0.11 | 1831 → 3152 | **0.219, 0.122** | 2.47× |
| EMB6 | dimension | 6 | 100% | **1.509 (0.992)** | 0.992 | 0.11 | 758 → 2014 | 0.038, 0.024 | 1.02× |
| EMB8 | dimension | 8 | 100% | **1.509 (0.992)** | 0.992 | 0.11 | 776 → 2032 | 0.037, 0.024 | 1.05× |
| EMB12 | dimension | 12 | 100% | **1.509 (0.992)** | 0.992 | 0.11 | 812 → 2068 | 0.035, 0.024 | 1.10× |
| EMB16 | dimension | 16 | 100% | **1.509 (0.992)** | 0.992 | 0.11 | 848 → 2104 | 0.034, 0.023 | 1.14× |
| DS8 | dimension | 8 | 100% | **3.380 (0.996)** | 0.996 | 0.08 | 1680 → 4493 | 0.028, 0.017 | 2.27× |
| SIZE_COF | covolume | 4 | 100% | −0.505 (0.885) | 0.885 | 0.09 | 2419 → 1997 | 0.024, 0.024 | 3.26× |
| SKEW_U | skew | 4 | 100% | 1.536 (0.598) | 0.599 | 0.09 | 2085 → 3373 | **0.170, 0.112** | 2.81× |
| FORM_EUCL | form | 4 | 100% | **2.017 (0.994)** | 0.994 | 0.07 | 993 → 2670 | 0.032, 0.021 | 1.34× |
| PREC_256 | precision | 4 | **33%** | 0.026 (0.004) | 0.006 | 0.09 | mixed | mixed | n/a |
| DELTA_75 | delta | 4 | 100% | 1.436 (0.993) | 0.993 | 0.09 | 705 → 1900 | 0.039, 0.024 | 0.95× |
| QARY8 | ignore | 8 | **0%** | **5.540 (0.994)** | 0.994 | 0.11 | 2765 → 7366 | 0.034, 0.020 | 3.73× |
| FIX8 | ignore | 8 | **0%** | −0.003 (0.003) | 0.013 | 0.09 | 102 → 99 | **0.154, 0.163** | 0.14× |

The product panel (I_sec · I_q, or I_sec ⊕ I_q for DS8) tells the same
story. H2_4 has slope 2.008 (R² 0.994), 991 → 2663 steps. EMB8 and
EMB12 keep that slope exactly, with a small additive cost (1.04× and
1.07× at P248). DS8 on the direct sum is slightly *cheaper* than the
4-D product (0.93×) and a bit shallower (slope 1.874), because it
reduces two smaller blocks rather than one ideal of norm N·q. SIZE_COF
is flat at about 2,660 steps. H1_4 and SKEW_U again inflate CV
(0.16 at P248). No arm shows a key η² above chance (0.07–0.11 against
a chance of about 0.10).

## 3. Does dimension change the steps-per-bit law, or the CV?

**No, not when only the dimension of the same problem is raised.**

On every one of 1,200 samples, the EMB* step count is the H2_4 count
plus a *constant* that depends only on n: +17, +35, +71, +107
iterations at dimensions 6, 8, 12, 16. The residual (and therefore the
CV's numerator) is identical to H2. The slope is identical, 1.509
steps/bit, R² = 0.992. A slight drop in CV (0.039 → 0.034 at P248) is
only the same spread over a slightly larger mean. Extra dimensions, in
this embedding, add a fixed-size-reduction overhead of about nine
iterations per extra axis. They do not spread the size dependence, and
they do not hide it.

DS8, which is two copies of a q-sized ideal reduced jointly, *amplifies*
the law: slope 3.38, about 2.25× the cost of H2_4, with a slightly
lower CV for the same reason (a larger mean). QARY8, an 8-D lattice
whose volume still tracks q but which does not solve the problem,
amplifies it further (slope 5.54, 3.73×). Raising dimension while
leaving volume tied to q makes the size leak louder, not quieter.

FIX8 is the control that *looks* flat (102 → 99 steps, slope 0, η² vs
size 0.013), and it is flat because it ignores the problem *and* has a
constant volume. Its CV is *worse* than H2 (0.15–0.16). Flatness by
ignoring is not consistency.

## 4. Which characteristic buys variability and consistency?

- **Fresh-lattice selection, not dimension, was the 2-D→4-D
  flattening.** H1_4, which keeps the D0 lattice and only changes the
  starting basis, is still 100% correct, but it is the worst of the
  solvable 4-D arms for consistency: CV 0.22 at P248, η² vs size only
  0.56 because that extra noise drowns the (still present) 1.56
  steps/bit slope. SKEW_U, the same operation on the H2 lattice,
  behaves the same way (CV 0.17, slope 1.54, R² 0.60). A worse
  starting basis adds cost and variance; it does not flatten size
  dependence. D0_4 is cheaper (0.31×) and has a shallower slope (0.46)
  because its HNF is a better-conditioned start for this particular
  ideal, not because the law is gone (R² is still 0.98).
- **Covolume is the only arm that removes the *growing* 1.5 steps/bit
  slope among exact, correct reductions.** SIZE_COF pads every set
  toward P664 volume, so P248 pays 3.26× its own H2. The count is then
  roughly a large-set count. CV stays low (~0.024), but this is the
  "move the cost" result of the salt-reshape experiment, not a
  size-blind algorithm.
- **The quadratic form can make the law *worse*.** FORM_EUCL still
  solves the exact nrd problem 100% of the time, but the slope rises
  to 2.02 steps/bit and the cost to 1.34×. The reduced-norm form is
  already the better-conditioned one for this lattice.
- **A weaker LLL parameter barely moves the picture.** DELTA_75 is 95%
  of the H2 cost, slope 1.44, same CV, still 100% correct. It is not a
  path to independence or to a flatter trace.
- **Low precision can look flat and still sometimes work, but it is
  not a method.** PREC_256 is exact on 33% of rows (397 of 1,200).
  Those successes are flat at 269 ± 19 steps (slope −0.004, CV 0.07)
  at every set. The other 67% abort because 256 bits do not cover the
  Gram's dynamic range. Raising the precision until the form is
  reliably definite brings the size of the integers, and the slope,
  back with it.
- **Nothing here is constant time**, and nothing showed a secret-key
  leak at this resolution. A fixed-schedule reduction remains the way
  to flatten the *count* of an exact, correct reduction.

## 5. Structural flags

- **Dimension is not a substitute for a fixed schedule.** Extending the
  same ideal into rank 6–16 leaves the 1.5 steps/bit law untouched
  and adds a constant overhead. Direct sums and q-ary lattices whose
  volume tracks q amplify it.
- **The 2-D→4-D drop in target dependence should not be read as a
  dimension effect.** In 4-D, H1 (same lattice, new basis) kept 21–26%
  target η² at fixed size; this ablation shows that the same mask also
  inflates CV and leaves the size slope in place. H2's *fresh ideal*
  was the flattening that mattered at fixed size. Size across sets is
  a different leak, and neither dimension nor a fresh ideal removes it.
- **Exact LLL of a geometrically faithful lattice tracks starting skew
  and volume.** Worse presentations (H1, SKEW_U) cost more and vary
  more. Larger volume (SIZE_COF, official sets) costs more. A
  similarity does not change the count (the previous SCALE result).
- **FIX8's flat trace is not a solution.** It does not solve the norm
  equation, and its CV is worse than H2's.
- **DS8 on I_sec ⊕ I_q is not a free lunch for the secret.** It is
  cheaper than reducing I_sec · I_q, because the blocks have norms N
  and q rather than N·q, but the secret block still has a
  key-dependent volume, and key η² was only at chance at this
  resolution.
- **Within-set bits(q) is constant.** The size leak that this design
  question is about is the leak *across* sets.
- **Model limits.** Simplified order O = ℤ⟨1, i, j, k⟩, not O_pk; step
  counts, not cycles; simulated keys. Product-panel tests detect η² of
  about 0.15 or larger; smaller secret leaks are open. This harness
  does not include a true Kani or product-isogeny lattice.

## 6. Files

`hdl.py`, `run_hd.py` (skips existing CSVs), `analyze_hd.py`,
`results/summary_hd.json`, `results/summary_hd_table.csv`,
`results/histograms/`, `plots/`. Raw per-sample CSVs in `results/raw/`
are regenerated by the runner.
