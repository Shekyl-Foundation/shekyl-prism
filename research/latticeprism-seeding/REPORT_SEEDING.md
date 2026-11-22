# Seed constructions for H2 ideal selection

Date: 2026-11-21. Research for Shekyl PRISM. Step-counting model, no
constant-time or security claim. The 4-D H2 result in
`../latticeprism-4d/` is the starting point: hash-selecting the ideal
flattens target dependence, but a public seed is recomputable.

**Coverage.** This write-up uses complete stream, product and fixed-q
panels at P248, P324 and P376 (50 keys × 200 messages; 24 × 150
fixed-q), plus the P500 stream (same 50 × 200). P500 product / reduced
fixed-q, and a reduced P664 panel (25 × 64, fixed-q 16 × 40), were
launched detached to finish after an interruption; those numbers will be
folded in when they land. The qualitative verdict is already stable
across the four finished streams.

## 1. What each construction actually does

All six feed the same H2 selector: rejection-sample y from
XOF(seed, q, "H2") until x² + y² ≡ −p (mod q), then LLL-reduce the
left ideal of norm q (and, in the key panel, the product I_sec · I_q).
They differ only in how the seed and the challenge q are formed. SHAKE-256
is H and the XOF. A simulated public key is H("pk" ‖ sk) of the set's
length. Fiat-Shamir ("FS") draws an odd s of e−4 bits until
q = 2^(2e−5) − s² is prime.

**B0, baseline.** q = FS(pk, msg). seed = H(pk ‖ q). The signature is an
ordinary PRISM signature. The verifier, and any attacker, recomputes q
from (pk, msg) and the seed from (pk, q), so the H2 index and the LLL
trace are public before signing.

**Hpk, public fold.** q = FS(pk, msg). seed = H(pk ‖ msg). Same public
recomputability, with the seed depending on the message rather than on q.
Included to show that "hash the public key with something else public"
does not help.

**Hsk, EdDSA-style.** q = FS(pk, msg), still public from (pk, msg)
alone. seed = H(sk ‖ msg ‖ q). The signature carries nothing extra. The
verifier recomputes q from (pk, msg) and checks the signature; it does
not recompute the seed or the ideal. An attacker who lacks sk cannot
recompute the trace, but does know q before the signer has finished.

**COM, commitment.** r = H(sk ‖ msg), c = H(r) goes in the signature,
q = FS(pk, c, msg), seed = r. The verifier recomputes q from (pk, c, msg)
and checks the signature. It cannot recover r from c. An attacker does
not know q, or the seed, until c is published, and never learns r if H is
one-way. This is ordinary Fiat-Shamir binding of the challenge to a
first message.

**EPH, ephemeral sub-key.** (esk, epk) are derived from H(sk ‖ msg); epk
goes in the signature; seed = H(epk ‖ msg); q = FS(pk, epk, msg). The
verifier recomputes q from (pk, epk, msg). After the signature is out,
anyone can recompute the seed from (epk, msg). Before signing, they
cannot.

**REV, reverse map.** From H(sk ‖ msg) the signer samples a unimodular
U first (independent of q), commits to d = H(U), and only then sets
q = FS(pk, d, msg). The signing ideal of norm q is selected with
seed = H(sk ‖ msg ‖ q), the Hsk formula. The signature carries d. The
verifier recomputes q from (pk, d, msg) and checks the signature; it
never sees U and does not rerun LLL. A pre-challenge LLL on U itself
(determinant 1, 128-bit entries) is timed separately; that step does
not include the secret ideal and does not produce the signing lattice.

## 2. Verdict table

Figures below are P248 unless a range is given. Every construction was
deterministic (same (sk, msg) gave the same seed, q, and H2 index in
100% of 10,000 stream rows per set) and the verifier recovered q in
100% of rows. H2 outputs were valid (T | nrd(β)) in 100% of rows.
Chance η² is about 0.0049 for 50 keys and about 0.0064 for 24 fixed q.

| | B0 | Hpk | Hsk | COM | EPH | REV |
|---|---|---|---|---|---|---|
| Deterministic | yes | yes | yes | yes | yes | yes |
| y/q uniform? (KS p, 32-bin χ² p) | 0.77, 0.54 | 0.11, 0.015 | 0.43, 0.50 | 0.034, 0.61 | 0.14, 0.51 | 0.36, 0.67 |
| Index collisions, 995,000 same-key pairs | 0 | 0 | 0 | 0 | 0 | 0 |
| Precomputable from (pk, msg) only | **100%** | **100%** | 0% | 0% | 0% | 0% |
| Precomputable after the signature | **100%** | **100%** | 0% | 0% | **100%** | 0% |
| Verifier can recompute q | yes | yes | yes | yes (needs c) | yes (needs epk) | yes (needs d) |
| η² of q-ideal trace vs key (KW p) | 0.003 (0.99) | 0.006 (0.06) | 0.005 (0.57) | 0.005 (0.27) | 0.005 (0.29) | 0.006 (0.16) |
| η² of I_sec·I_q trace vs key | 0.006 (0.037) | 0.004 (0.52) | 0.006 (0.17) | 0.005 (0.35) | 0.006 (0.29) | 0.006 (0.11) |
| η² of q-ideal trace vs a *fixed* q | 0.006 (0.61) | 0.004 (0.93) | 0.007 (0.24) | 0.005 (0.67) | 0.006 (0.31) | 0.007 (0.24) |
| Cost vs D0 (q-ideal LLL) | 3.21× | 3.21× | 3.21× | 3.21× | 3.21× | 3.21× |
| Trace CV (constant-time?) | 0.041, no | 0.041, no | 0.041, no | 0.041, no | 0.040, no | 0.041, no |

The same pattern holds at P324, P376 and the P500 stream: cost 3.23×,
3.24× and 3.26× D0; H2 traces 973 ± 35, 1125 ± 37 and 1500 ± 43; every
secret-seed construction still at chance η²; B0 and Hpk still 100%
precomputable; EPH still 100% precomputable once epk is published; zero
index collisions. D0's trace at a fixed q has η² = 1, as it must: it is
a function of q alone. REV's pre-challenge map LLL is 24.4 ± 7.8 steps
at every set (the lattice is U itself, determinant 1, independent of p
and of q), CV 0.32, and is also not constant time.

Mild uniformity p-values (Hpk χ² p = 0.015 at P248, 0.049 at P324; COM
KS p = 0.034 at P248) are consistent with running many tests; they do
not repeat at P376 or P500.

## 3. Recommendation

The thing that works is **a seed that includes the long-term secret,
bound, if the protocol needs it, to a commitment that the challenge
hashes**. Hsk, COM and REV all give deterministic, uniform-looking,
not-publicly-precomputable H2 selection. None is constant time.

- Use **COM**, or **Hsk with COM's Fiat-Shamir** (seed = H(sk ‖ msg ‖ q)
  and q = FS(pk, c, msg)), when the challenge should not be known before
  a first message is bound. That is the usual reason to have a
  commitment, and it is independent of the lattice.
- **Hsk alone** is the simplest deterministic secret seed, and it is
  enough against a timing adversary who knows only pk and msg. Its q is
  public from (pk, msg) before signing. If that is a problem for PRISM's
  identification transcript, add the commitment; do not expect the
  lattice layer to fix it.
- Do not use B0 or Hpk. Do not use EPH: it looks like a fresh key and
  then publishes the seed.

None of these replaces a fixed-tour constant-time LLL (TCHES 2025) for
the reduction itself. The seed only hides *which* ideal is reduced.

## 4. Are REV and COM different from Hsk, or the same thing relabelled?

They are **distinct in the Fiat-Shamir transcript, and the same in the
signing-ideal traces.**

Hsk makes q a public function of (pk, msg). COM and REV do not: each
puts a 32-byte hash in the signature (c = H(r) or d = H(U)) and sets
q = FS(pk, commitment, msg). That is a real difference, and it is why
an attacker who has not seen the signature cannot even form q, let
alone the seed, for COM or REV.

COM and REV are close cousins. Both are "commit to a secret-derived
value, then derive q." They differ in *what is committed*:

- COM commits to r, and r *is* the H2 seed. There is no lattice until
  after q exists.
- REV commits to a unimodular map U that is sampled *before* q, then
  selects the signing ideal with Hsk's seed formula H(sk ‖ msg ‖ q).
  The pre-challenge LLL on U does not enter verification except as the
  preimage of d, which the verifier never opens.

So REV, as implemented, is a **combination**: COM-style binding of q,
plus Hsk-style ideal selection, plus a map-LLL that is independent of
the target and of the secret ideal. For the H2 traces that actually
solve the norm-q problem, REV, COM and Hsk are statistically
indistinguishable (means agree to a fraction of a step). The reverse
idea — "sample the map first, derive the challenge from it" — is real,
but in this protocol it amounts to using H(U) as COM's commitment. It
does not, by itself, give a constant-time or privately timed signing
reduction.

## 5. Structural flags

- **Deterministic + uniform is pseudorandom-from-secret**, not true
  randomness. That is the only sense in which the requirement can be
  met, and Hsk/COM/REV meet it.
- **No construction is constant time.** The H2 LLL still varies by
  about ±30 to ±43 steps (CV 3–4%). A public seed would let an attacker
  recompute that trace; a secret seed only hides which trace occurred.
  Equalizing the *count* still needs a fixed schedule.
- **Covolume still scales the mean** across sets (743, 973, 1125, 1500
  steps at P248–P500), about 1.5 iterations per bit of q, as in the 4-D
  report. Harmless while every signature at a set has the same public
  |q|.
- **EPH leaks the seed in the signature.** Publishing epk is publishing
  the input to seed = H(epk ‖ msg).
- **Hsk's q is public pre-signature.** That is not a timing leak of the
  secret (η² vs key is at chance; η² vs a fixed q is at chance for H2),
  but it is a transcript choice, not a lattice one.
- **REV's map-LLL does not solve the signing problem** and does not see
  the secret. Any later reduction of I_sec · I_q still has to be
  addressed (the product panel, so far, shows no key dependence at this
  resolution, including for D0-style secrets re-randomized by a fresh q).
- **Verifier integration.** The verifier of PRISM checks the signature,
  not the internal H2 path. COM/EPH/REV add 32 bytes. Nothing here
  replaces RepresentInteger's retry loop, which remains a much larger
  variable-time step if it is still used downstream.
- **Model limits.** Simplified order O = ℤ⟨1, i, j, k⟩, not O_pk;
  step counts, not cycles; simulated keys. The product-panel test
  detects η² of about 0.02 or larger; smaller leaks are open. P664 and
  the P500 product panel were still running when this section was first
  written.

## 6. Files

`constructions.py`, `run_seeding.py` (skips existing CSVs),
`analyze_seeding.py`, `results/summary_seeding.json`,
`results/summary_seeding_table.csv`, `results/histograms/`, `plots/`.
Raw per-sample CSVs in `results/raw/` are regenerated by the runner.

## 7. Addendum: P500 product panel and P664 (reduced panel)

All five sets have now finished. P664 used a smaller panel (25 keys × 64
messages; fixed-q 16 × 40), so its chance η² is higher: about 0.015 for
the key panels and about 0.023 for fixed q. Every P664 value sits in that
range (key η² 0.009–0.025, fixed-q 0.013–0.036). The verdict doesn't
change: all six constructions are deterministic with zero index
collisions, and the uniformity p-values are 0.14–0.89. B0 and Hpk are 100%
precomputable, EPH is 100% precomputable after signing, and Hsk, COM and
REV are 0% precomputable. H2 traces are 1995 ± 49 at about 3.27× D0, so
none is constant time. Across the five sets the mean scales at about 1.5
iterations per bit of q (743, 973, 1125, 1500, 1995). Per-set figures are
in `results/summary_seeding_table.csv`.
