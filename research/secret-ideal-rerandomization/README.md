# Re-randomizing the secret ideal per signature (research harness)

Research harness for Shekyl PRISM. PRISM is the scheme of ePrint
2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi, Leroux,
Maino, Vercauteren, Wesolowski); shekyl-prism implements it. This
directory does not change the signer, the signature format, or any
parameter set. Status: **in progress**; results will go in `REPORT.md`.

## Question

The secret ideal I (norm N) has many equivalent representatives
J = I · β̄ / N for β ∈ I, all with the same left order and a right order
conjugate to O_R(I), i.e. the same key curve up to isomorphism. If the
signer draws a fresh J for every signature, the signing-path reduction
starts from a lattice whose size is set by the fresh norm N' = nrd(β)/N,
not by the long-term N. Does that remove the key-dependent channel that
`research/fixed-schedule-reduction` found in exact LLL (about 2 steps per
bit of N), and is it worth having as defence in depth on top of the
fixed Z[i]-Lagrange schedule?

## Plan

a. **Sampling β.** Reduce I once at key generation (Z[i]-Lagrange fixed
   schedule sized from the bound B = ⌈L/2⌉+32), keep the reduced basis
   {u, iu, v, iv}, and per signature take β = Σ cᵢ bᵢ with small random
   coefficients. Variants: plain vs shape-normalized coefficient ranges;
   accept N' prime vs accept any odd N' whose ideal has the standard
   cyclic shape.
b. **Cost and success.** Candidates per signature, primality tests,
   failure rate, and the fixed candidate count K needed for a
   constant-time version with failure ≤ 2⁻⁶⁴.
c. **Downstream independence.** Norm-size distribution of N'; exact LLL
   and the fixed Z[i] schedule on the re-randomized product versus the
   baseline product I ∩ I_q, for the same keys and messages: slope and
   correlation of LLL steps against bits(N) and key shape, between-key
   η², and a dudect-style fixed-key vs random-key Welch t-test.
d. **Does the sampler leak?** Its retry count and N' size against key
   features and in the fixed-vs-random t-test.
e. **Bookkeeping.** What the signer has to map back so verification is
   unchanged, modelled at the ideal level (the re-randomized short
   element mapped back to the original product).

Same conventions as the baseline: simulated keys in the index-4 order
Z⟨1, i, j, k⟩, operation counts not cycles, Python big integers are not
constant-time. Sets: P248, P324, P376, P500, P664 only.

## Files

| file | what |
|---|---|
| `hdl.py`, `fsr.py`, `run_fsr.py` | Copied unchanged from `research/fixed-schedule-reduction` (keys, challenge, H2 ideal, exact LLL with counters, Z[i]-Lagrange fixed schedule). |
