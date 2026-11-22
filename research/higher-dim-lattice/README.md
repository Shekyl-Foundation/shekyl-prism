# research/higher-dim-lattice

**This is a research experiment for Shekyl PRISM. PRISM is the signature
scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin, Wouter Castryck,
Maria Corte-Real Santos, Riccardo Invernizzi, Antonin Leroux, Luciano
Maino, Frederik Vercauteren, and Benjamin Wesolowski; the paper is at
`docs/papers/2026-443_PRISM_with _salt.pdf`). Shekyl's `shekyl-prism` is
an implementation of it. The experiment's code, data and conclusions,
including negative ones, are public by design.** It is not part of any
crate, CI does not build or test it, and it makes no constant-time or
security claim. Findings are in [REPORT.md](REPORT.md).

This is research, not a proposal to adopt a new signature format.

The Shekyl parameter sets, and the only ones run, are P248, P324, P376,
P500 and P664, as defined in `prism-field`. P363 and P384 are not
Shekyl parameter sets and are not used.

## Purpose

A previous 4-D experiment found that choosing a *fresh* ideal of norm q
(H2) flattened the target-dependence seen in 2-D, while a unimodular
mask of a fixed 4-D ideal (H1) kept 21–26% of the trace variance. The
size of the lattice still leaked at about 1.5 steps per bit of q. A
follow-up that *padded* the same 4-D grid to P664 volume moved the cost
but did not make LLL size-blind.

This study asks whether *raising the dimension* of the lattice, rather
than its volume, changes that steps-per-bit law or the trace CV — and,
in the same harness, which other lattice characteristic actually buys
target-independence and consistency. The 2-D→4-D jump is tested as two
separate effects: dimension, and fresh-lattice selection (H2 vs H1/D0).

The study is a **one-characteristic-at-a-time ablation** against the
4-D H2 baseline. Each arm changes one of: dimension, covolume, starting
skew, the quadratic form, arithmetic precision, the LLL parameter
delta, or (as a control) which ideal is selected. Two further arms are
higher-dimensional lattices that do *not* solve the norm equation, to
show flatness-by-ignoring.

## Salt model

The challenge is computed from the public key, a 32- to 96-byte salt
carried in the signature, and the message, as in Shekyl PRISM. Salt
length is `width_bits(e)/8`. Here the salt is
SHAKE-256("salt" ‖ sk ‖ msg) truncated to that length, and
`q = FS(pk, salt, msg)`. After signing, the salt and q are public.
Challenge-dependent timing leaks nothing secret. What needs constant
time is the work on the secret ideal and the norm equations, modelled
only roughly here.

## Arms

The baseline (H2_4) is a hash-chosen left ideal of norm q, reduced
with Cohen's integral LLL (δ = 99/100) under the reduced-norm form,
starting from the HNF basis, in exact integer arithmetic.

| Id | Characteristic changed | Dim | What is reduced, and what a short vector is supposed to give |
|---|---|---|---|
| H2_4 | — (baseline) | 4 | H2 ideal of norm q. A short vector β should satisfy q \| nrd(β) and lie in the ideal. |
| D0_4 | selection | 4 | Deterministic smallest-y ideal of the same q. Same problem. |
| H1_4 | selection | 4 | D0's ideal, starting basis masked by a 128-bit unimodular (the 4-D H1 control). Same lattice as D0, not a fresh ideal. |
| EMB6, EMB8, EMB12, EMB16 | dimension | 6–16 | The *same* H2 ideal, extended by salt-chosen extra generators in the ideal plus fixed-scale (2^{16}) extra axes, independent of q. A reduced vector's first four coordinates should lie in the ideal and satisfy q \| nrd. |
| DS8 | dimension | 8 | Direct sum of two H2 ideals of the *same* q (a second class from a salt-shifted seed). A reduced vector should have a 4-block in one of the two ideals. This is a block embedding in O ⊕ O, not a Kani correspondence. |
| SIZE_COF | covolume | 4 | Salt-chosen cofactor so r·q ≈ 2^{1323}; reduce I_q I_r. Same membership problem for q. |
| SKEW_U | starting skew | 4 | The same H2 lattice as the baseline, starting basis multiplied by a 128-bit unimodular. |
| FORM_EUCL | quadratic form | 4 | Same H2 lattice, LLL under the Euclidean form instead of the reduced norm; the unimodular is applied to the exact basis and checked under nrd. |
| PREC_256 | precision | 4 | Same lattice and form, Gram quantized to 256-bit entries. |
| DELTA_75 | LLL delta | 4 | Same lattice, form, and start; δ = 3/4 instead of 99/100. |
| QARY8 | ignore (control) | 8 | An 8-D q-ary lattice of modulus q. Encodes nothing about the norm equation. Volume still tracks q. |
| FIX8 | ignore (control) | 8 | The same 8-D q-ary construction with a *fixed* modulus 3329. Volume independent of q. |

The product panel repeats the solvable arms on I_sec · I_q (and, for
DS8, on I_sec ⊕ I_q), which is the secret-touching lattice.

## Running

Same Sage 10.x environment as the earlier research harnesses. The
runner skips any CSV that already exists.

```sh
micromamba run -n sage python run_hd.py \
    --keys 24 --msgs 10 --keys-prod 24 --msgs-prod 10 \
    --procs 7 --out results/raw
micromamba run -n sage python analyze_hd.py
```

## Review notes (Rick, 2026-11-21)

These experiments are a measuring tool for the hardening stage, not
the design of a LatticePrism type now. The scheme already has a salt
in the challenge hash; the challenge is public, so challenge-dependent
timing leaks nothing secret; what needs constant time is the work on
the secret ideal and the norm equations; the signing path has no
retry-heavy integer-representation step (stage 1a); and format-changing
constructions are a different scheme, not to be adopted without a
design round.
