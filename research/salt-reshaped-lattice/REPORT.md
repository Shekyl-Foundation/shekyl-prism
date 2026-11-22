# Salt-reshaped 4-D grids: can the size–step link be cut?

Date: 2026-11-22. Research for Shekyl PRISM. Step-counting model, no
constant-time or security claim. This is not a proposal to change the
signature format or the challenge hash.

The 4-D work in `../latticeprism-4d/` (on the sibling experiment
branch) found that LLL on a left ideal of norm q costs about 1.5 steps
per bit of q. The question here is whether the salt can *reshape* that
grid — not just pick which ideal of the same volume is used — so the
step count no longer tracks the challenge size.

**Coverage.** All five Shekyl sets (P248, P324, P376, P500, P664), 32
keys × 16 messages per set for the I_q stream and for the I_sec·I_q
product panel (512 rows per construction per set), plus a synthetic
q-bit sweep (400 to 1323 bits, 30 primes each) at fixed p for P248 and
P664. Salt is SHAKE-256("salt" ‖ sk ‖ msg) of length 32, 48, 48, 64 or
96 bytes, and q = FS(pk, salt, msg), as in the scheme.

## 1. What each construction does

The quaternion order, the H2 selector, and Cohen's integral LLL are
those of the 4-D harness. The signer always derives the salt from
(sk, msg) and puts it in the Fiat-Shamir hash. After signing, the salt
and q are public.

- **H2.** Baseline: reduce the hash-chosen left ideal of norm q.
- **D0.** Same q, deterministic smallest-y ideal, for cost comparison.
- **SCALE.** Multiply the H2 basis by an integer λ so the norm-metric
  covolume |det B|·p matches a P664-sized target, run LLL, and scale
  back. The lattice is a homothety of the original: same shape, larger
  integers.
- **COF.** From the salt, pick a prime r so that r·q is about 2^{1323}
  (P664's typical challenge). Reduce the product ideal I_q I_r, whose
  short vectors still lie in I_q. If the published degree became r·q,
  that would be a different scheme; here it is an internal test.
- **SUPER.** Coarsen I_q to the only constant-determinant superlattice
  inside O ≅ ℤ⁴. The HNF diagonals of I_q are (q, q, 1, 1), so the only
  exact coarsenings have index q or q²; we take det 1, essentially O
  itself, with a salt-independent HNF shrink.
- **Q256.** Quantize the Gram matrix to 256-bit entries, LLL that form,
  and apply the unimodular transform to the exact basis.
- **H2PROD, PCOF, COMP.** The same ideas on the secret-touching product
  I_sec·I_q. PCOF's cofactor is a function of the published salt and q
  only, so it cannot cancel the secret norm N. COMP chooses r from
  (sk, N) so that N·q·r is about 2^{1663}, covering a P664 product;
  r is not recoverable from the published salt. Neither changes the
  signature format.

## 2. Verdict

Figures are means ± one standard deviation, 512 samples, unless noted.
"Slope" is a least-squares fit of iterations against bits of q across
the five sets (2,560 stream rows per construction). Within a set,
bits(q) does not vary at all: q = 2^{2e−5} − s² always has bit length
2e−5, so there is no within-set size leak to remove, only a residual
CV of 2.6–3.9% at fixed size.

| | H2 | D0 | SCALE | COF | SUPER | Q256 | H2PROD | PCOF | COMP |
|---|---|---|---|---|---|---|---|---|---|
| Solves the exact problem? | 100% | 100% | 100% | 100% | **0%** | **36%** | 100% | 100% | 100% |
| Slope, steps per bit of q (R²) | **1.505 (0.992)** | 0.455 (0.979) | **1.505 (0.992)** | −0.510 (0.889) | 0 (flat) | 0.005 (0.000) | **2.011 (0.994)** | 0.002 (0.000) | −0.496 (0.848) |
| Mean steps P248 → P664 | 742 → 1994 | 232 → 611 | 742 → 1994 | 2420 → 1994 | 3 → 3 | ~95 mixed | 995 → 2669 | 2670 → 2669 | 3098 → 2686 |
| Trace CV (P248, P664) | 0.039, 0.026 | 0.064, 0.042 | 0.039, 0.026 | 0.023, 0.026 | 0 | mixed | 0.035, 0.022 | 0.021, 0.022 | 0.020, 0.022 |
| Cost vs H2 at P248 | 1.00× | 0.31× | 1.00× | 3.26× | 0.004× | n/a | — | — | — |
| Recomputable after signing? | yes (I_q) | yes | yes | yes | yes | yes | no (I_sec) | no | no |
| Changes the format? | no | no | no | if published | no, but wrong output | no, when it works | no | no | no |

SCALE and H2 traces are **identical** at every sample (max absolute
difference 0). Exact integer scaling does not change a single Lovász
or size-reduction decision. Homothety cannot break the size leak.

COF and PCOF do stop the *growing* 1.5–2 steps/bit slope, but only by
padding every lattice toward P664 volume. P248 then costs 3.26× its
own H2 (2,420 steps vs 742), which is more than P664 H2 itself (1,994),
because a composite-norm product I_q I_r is a worse-conditioned
starting basis than a prime-norm ideal of the same size. The residual
COF slope is slightly negative and almost perfectly linear on the set
means (R² = 0.9998): as r shrinks toward 1, the lattice looks more like
plain H2. PCOF is empirically flat at 2,669–2,674 steps across all five
sets (slope 0.002, R² = 0.000). COMP, which also tries to cancel the
secret norm N, is a bit more expensive at the small sets (3,098 at
P248) and is not flatter than PCOF.

SUPER is perfectly flat at exactly 3 iterations on every one of 2,560
samples, and never returns a vector in I_q. That is the only exact
constant-determinant superlattice in the order, and it is essentially
the order itself.

Q256 stays positive-definite, and then still solves the exact problem,
on 912 of 2,560 samples (35.6%). Those successes have a **flat** cost
of 268 ± 18 steps at every set (slope −0.0001). The other 64.4% abort
before LLL because 256 bits do not cover the dynamic range of the true
Gram matrix (about 1,200–3,300 bits). A signing method that fails two
thirds of the time is not usable, and raising the precision until the
form is reliably definite brings the size of the integers — and the
step count — back with it.

The synthetic sweep, at fixed p, confirms that the slope is a property
of q-size, not of switching sets: H2 is 2.02 steps/bit of q at P248 and
2.01 at P664 (R² ≥ 0.995), D0 is 0.61 steps/bit at both. The across-set
H2 slope is a little shallower (1.50) because p and q grow together in
the official sets, which is a slightly better-conditioned family than
a huge q at a small p.

η² of the product-panel traces against the key is 0.04–0.07 at every
construction, with Kruskal–Wallis p-values 0.15–0.91. Chance η² for
32 keys × 16 messages is about 0.061. No construction showed a
detectable secret-key leak in the step count at this resolution. None
is constant time: even the padded traces have CV about 2%.

## 3. Can the size dependence be removed?

**Not in the exact geometry, without moving the cost or dropping the
problem.** The covolume of a left ideal of norm T, in the reduced-norm
metric, is |det B|·p, and T is q or N·q (or a padded product). Exact
LLL of that lattice sees the starting HNF's skew, which grows with T.
Cohen's algorithm is invariant under exact homothety, so scaling the
grid to a standard volume leaves the step count untouched (SCALE).
The only exact constant-volume superlattice inside the order no longer
contains the ideal (SUPER). The things that *do* flatten the count are:

1. **Pad to the largest volume** (COF, PCOF, COMP). Every set pays at
   least a P664-sized reduction, sometimes a little more for a
   composite-norm basis. The 1.5–2 steps/bit growth is gone because
   the lattices are all large, not because the algorithm became
   size-blind. This can be done internally on I_sec·I_q with no
   format change (PCOF, COMP). It does not give a cheap small-set
   reduction, and it is not constant time.
2. **Reduce a different, coarsely quantized form** (Q256). Steps can
   be flat when the quantized Gram stays definite, but most of the
   time it does not, and enough precision to be reliable is enough
   precision to see the original skew again.

A salt-only cofactor cannot cancel the secret norm N, because after
signing the salt is public. COMP can, in principle, because r is
derived from sk and N; in this panel it was not better than PCOF.

What still needs constant time is the work on the secret ideal and the
norm equations. Padding the public-q lattice does not change that.
A fixed-schedule reduction (TCHES 2025, or a fixed-tour LLL) remains
the way to flatten the *count* of an exact reduction without changing
the lattice.

## 4. Structural flags

- **Deterministic + uniform is not the issue here.** The salt is already
  a deterministic secret-derived value in the challenge hash. The leak
  is the *volume* of the lattice being reduced.
- **Exact LLL is scale-invariant.** Any reshape that is a homothety, or
  a unimodular change of the same lattice, will keep the 1.5 steps/bit
  law. That is the 2-D masking result, again, in a new language.
- **Covolume is |det B|·p, not just q.** A cofactor that freezes q (or
  r·q) still leaves p, and N on the product side, in the volume. In
  this data the leftover growth was small compared with padding, and
  for PCOF it happened to cancel, but it is not a proof that p or N
  cannot appear in a more sensitive metric (cycles, limb operations).
- **COF, if published as a degree-r·q isogeny, is a different scheme**
  with no proof, and would leave the C-reference replay test. As an
  internal padding of I_sec·I_q it does not change the format.
- **SUPER's flat trace is the trace of reducing O**, not of solving the
  signing problem. Recovery of an I_q vector from a short vector of O
  is a CVP, which moves the work and likely the size dependence.
- **Q256 is not a reliable algorithm.** 36% success, 64% abort. The
  successes are encouraging as a *hint* that a bounded-precision form
  can be size-blind, and discouraging as a method.
- **Within-set bits(q) is constant.** The official challenge
  q = 2^{2e−5} − s² never changes bit length. The size leak that
  matters for this design question is the leak *across* sets, and
  the general law that a larger public volume costs more steps.
- **Model limits.** Simplified order O = ℤ⟨1, i, j, k⟩, not O_pk;
  step counts, not cycles; simulated keys. Product-panel tests detect
  η² of about 0.08 or larger; smaller secret leaks are open.

## 5. Files

`srl.py`, `run_reshape.py` (skips existing CSVs), `analyze_reshape.py`,
`crosscheck_reshape.py`, `results/summary_reshape.json`,
`results/summary_reshape_table.csv`, `results/histograms/`, `plots/`.
Raw per-sample CSVs in `results/raw/` are regenerated by the runner.
