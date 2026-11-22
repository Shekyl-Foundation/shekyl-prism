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

## In progress

| Folder | Question | Status | Source |
|--------|----------|--------|--------|
| *(not yet on this branch)* | Does a reduction with a schedule set by the parameter set, or a fixed-budget enumeration or sieve, flatten the exact-LLL count? Tour counts are to be sized from the bound B = ⌈L/2⌉+32, not from the secret norm N. The secret product is to be measured across keys whose N has different lengths. D0 is excluded. | In progress on its own public experiment branch. It will be folded into this branch later. | `experiment/fixed-schedule-reduction` |

## Conclusions

From Rick's review of 22 November 2026, of the salt-reshape and
higher-dimension experiments, and of the earlier LatticePrism work they
rest on:

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
3. **The cross-set slope is not a leak.** Within a set, bits(q) is
   constant, and the parameter set is public. The reports' "size leak
   across sets" is therefore not a secret channel.
4. **The secret norm's length is a real channel.** The secret ideal's
   norm N varies in length from key to key (signer specification
   section 2.1: typical L/2+16, bound B = ⌈L/2⌉+32). The reduction of
   I_sec ∩ I_q, modelled here as I_sec · I_q, then leaks about 2 steps
   per bit of N. The fixed schedule's tour count must be sized from B,
   not from the actual N.
5. **D0 must not be used.** The deterministic smallest ideal is about
   three times cheaper, but the response ideal has to be uniform among
   the ideals of norm q; the proof and ledger O4 rest on it. A hash of
   the secret choosing uniformly (H2 with a secret seed) is
   acceptable; "smallest" is not.
6. **Statistical power is low.** Panels of 24 to 32 keys detect η² of
   roughly 0.08 to 0.15. "No key leak seen" means little at this
   resolution. A stronger test (fixed-key against random-key classes,
   a t-test, thousands of traces, on the real Rust code) belongs after
   stages 2 to 4.
7. **The retry loops are unmeasured.** The lattice of the challenge is
   public. The secret-touching work is key generation's reduction and
   its search for a prime-norm equivalent; signing's intersection and
   reduction; the norm equation's search (17-bit generators, 20,000
   steps, 400,000-pass budget, two uncapped inner loops); the
   translation to an isogeny; and curve arithmetic with secret scalars
   and kernels. Those retry loops are far larger channels than LLL's
   step count, and none of the five experiments here measures them
   against the key.

The quantized-precision runs (Q256, PREC_256) were sometimes flat where
they worked, which hints that bounded-precision reduction is size-blind
— the direction the constant-time LLL literature takes. They failed two
times in three, or more, so that is a hint and not a method.

The model throughout is the index-4 order with simulated keys, and only
LLL is modelled. Step counts are not cycles.
