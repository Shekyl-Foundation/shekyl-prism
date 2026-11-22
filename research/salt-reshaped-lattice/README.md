# research/salt-reshaped-lattice

**This is a research experiment for Shekyl PRISM. PRISM is the signature
scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin, Wouter Castryck,
Maria Corte-Real Santos, Riccardo Invernizzi, Antonin Leroux, Luciano
Maino, Frederik Vercauteren, and Benjamin Wesolowski; the paper is at
`docs/papers/2026-443_PRISM_with _salt.pdf`). Shekyl's `shekyl-prism` is
an implementation of it. The experiment's code, data and conclusions,
including negative ones, are public by design.** It is not part of any
crate, CI does not build or test it, and it makes no constant-time or
security claim. Findings are in [REPORT.md](REPORT.md).

This is research, not a proposal to adopt a new signature format or a
new challenge hash. Three of the constructions change what is reduced
internally, and one of them would change the degree that is published if
it were taken as a scheme. None of that should be adopted without a
design round.

## Purpose

Earlier 4-D work found that LLL on a left ideal of the challenge norm
costs about 1.5 steps per bit of the challenge, across the Shekyl
parameter sets, matching the kind of size leak reported for SQIsign 3.0.
Masking the basis, and hashing to *pick which* ideal of that norm is
used, did not cut the link between the prime's size and the lattice's
covolume.

The question here is whether the salt can *reshape the 4-D grid itself*
so that every lattice in a set — and, if possible, across the sets — has
one standard covolume, and the step count no longer scales with the
challenge.

The Shekyl sets, and the only ones run, are P248, P324, P376, P500 and
P664, as defined in `prism-field`. P363 and P384 are not Shekyl
parameter sets and are not used.

## Salt model

In Shekyl PRISM the challenge is computed from the public key, a 32- to
96-byte salt carried in the signature, and the message. The salt length
is `width_bits(e)/8`: 32, 48, 48, 64 and 96 bytes at P248..P664. In this
harness the salt is derived as SHAKE-256("salt" ‖ sk ‖ msg) truncated to
that length, and `q = FS(pk, salt, msg)`, so the salt is inside the
challenge hash, as in the scheme. After signing, the salt and the
challenge are public. Challenge-dependent timing leaks nothing secret.
What needs constant time is the work on the secret ideal and the norm
equations; those are modelled only roughly here. Stage 1a found that our
signing path has no retry-heavy integer-representation step; this
harness does not assume one on the live signing path.

## Constructions

All I_q-side constructions start from the same H2 ideal of norm q,
except D0, which uses the deterministic smallest-y ideal.

| Id | What the signer reduces | Signature format |
|---|---|---|
| H2 | Baseline: the H2 ideal of norm q. | Unchanged. |
| D0 | Deterministic smallest-y ideal of norm q. | Unchanged. |
| SCALE | Exact integer scale of the H2 basis so the norm-metric covolume matches a P664-sized target, then LLL, then divide the scale back out. | Unchanged. Internal only. |
| COF | Salt-chosen prime cofactor r so that r·q is about 2^{1323} (P664's typical q). Reduce I_q I_r. | Internal degree r·q. Publishing that degree would be a different scheme. |
| SUPER | The only constant-determinant superlattice of I_q inside O ≅ ℤ⁴, essentially O itself (HNF diagonals of I_q are (q, q, 1, 1)). | Unchanged as a signature, but the short vector is not in I_q. |
| Q256 | Det-style quantization of the Gram matrix to 256-bit entries, LLL of that form, unimodular applied to the exact basis. | Unchanged if it still solves the exact problem. |
| H2PROD | Baseline product I_sec · I_q. This is the secret-touching lattice. | Unchanged. |
| PCOF | Salt-chosen r (from the published salt and q only) times I_sec · I_q. Cannot cancel the secret norm N. | Unchanged (internal). |
| COMP | Secret-compensating r, chosen from sk and N so that N·q·r is about 2^{1663}, covering a P664 product. Not recomputable from the published salt. | Unchanged (internal). |

## Running

Same Sage 10.x environment as the earlier research harnesses (gmpy2,
numpy, scipy, pandas, matplotlib). The runner skips any CSV that already
exists.

```sh
micromamba run -n sage python run_reshape.py \
    --keys 32 --msgs 16 --keys-prod 32 --msgs-prod 16 \
    --sweep-draws 30 --procs 7 --out results/raw
micromamba run -n sage python analyze_reshape.py
```

Sample sizes: 32 keys × 16 messages per set for the stream and the
product panel; a synthetic q-bit sweep (400..1323 bits, 30 primes each)
at fixed p for P248 and P664, to separate the q-size slope from the
change of set.

## Review notes (Rick, 2026-11-21)

These experiments are worth keeping as a measuring tool for the
hardening stage, not as the design of a LatticePrism type now.

- The scheme's challenge is not known before signing, because of the
  salt. Challenge-dependent timing leaks nothing secret.
- The "commitment" in the earlier seeding experiment is essentially
  that existing salt, chosen from the secret key and the message.
- What needs constant time is the work on the secret ideal and the
  norm equations, modelled only roughly here.
- The signing path has no retry-heavy integer-representation step
  (stage 1a).
- Constructions that change the signature format or the challenge hash
  are a different scheme, with no security proof, and would fall
  outside the C-reference replay test. They are framed here as
  research, not adoption.

Intended later use: from stage 2 on, point a harness at the
secret-touching steps in each specification section, using our
constants and real secret ideals. In stage 7, use it as the first
filter, then port instrumentation to the Rust code, because step
counts are not cycles.
