# Ledger

Three lists: claims we may publish, departures from ePrint 2026/443, and open obligations. Status is **Ran** (executed, output committed), **Derived** (argued, not machine-checked) or **Open**. Update a row in the same PR that changes its evidence.

## Claims

| # | Claim | Status | Evidence | Reproduce |
| --- | --- | --- | --- | --- |
| C1 | The midpoint of Algorithm 3 can be identified by two half-chains with dual theta structures when the chain is two steps shorter than the torsion and the signature carries order-`2^e` points. | Ran | Sage model and Rust agree; per-step null points match on six vectors; every committed honest signature accepted. | `cargo test -p prism-verify`; `scripts/sage/s5_model.sage` |
| C2 | The two symplectic matrices are closed-form in the challenge `s` and `s⁻¹ mod 2^e`. | Derived (algebra), Ran (in use) | The identities follow from `q ≡ −s² (mod 2^e)`; a 6,000-sample random check is reported by the reviewer and not committed. In the tree: chain verdicts equal the library completion's on 73 honest signatures and every negative row (`bf20794`), and the verifier uses the closed form on all 113 vectors. | `cargo test -p prism-isogeny`; `scripts/sage/s5_model.sage` |
| C3 | Checking `Φ(X,0)` on `E_vk[2^e]` is enough to fix the top-left entry to `±s`; no `E_vk[p+1]` basis is needed. | Derived | Norm bound in `SECURITY_ARGUMENT.md` §4; inequality checked by `scripts/security_parameters.py`. | `python3 scripts/security_parameters.py` |
| C4 | The second component of that check is what binds the top two bits of the signature points. | Ran | The `[2^{e−1}+1]` scaling passes every earlier check and is refused there. | acceptance tests |
| C5 | Without a canonical Montgomery model for `E_sig`, one signature has several valid encodings. | Ran | With the check off in the model, the other five models accept. | model negatives, row 13 |
| C6 | A Weil-pairing identity cannot validate a signature vector: it holds for every determinant +1 image of the true preimages. A determinant −1 image inverts the pairing and is refused. | Ran | Model negative row 14: `(P, Q+P)`, `(P+Q, Q)` and `(Q, −P)` pass the pairing check and are refused at the codomain comparison. The verifier's tests refuse a swap of the two points at the pairing. The faulty export of 2026-11-10 passed the model's checks of that date and failed the chains; see the correction under that notebook entry. | model negatives, row 14; `cargo test -p prism-verify`; notebook 2026-11-10; `177eae5` |
| C7 | Unsalted, with a public index, collision replay costs about `2^125 / 2^190 / 2^252` hash queries. | Ran (density), Derived (exponent), Ran (slope at reduced width) | `docs/comparison/e1`, `e3` | `scripts/run_comparison.sh`, `scripts/analyze_comparison.py` |
| C8 | With the salt, the same attacker needs a second preimage; cost slope near 1. | Ran (reduced width), Derived (full width, from the paper's eq. 12) | `docs/comparison/e3`; `SECURITY_ARGUMENT.md` §6 | as C7 |
| C9 | The pinned C reference divides by zero in `ibz_cornacchia_prime` on a negative cofactor; key generation aborts on every sampled seed at level III. | Ran | Abort classification and the routine's analysis in the model report. Key generation aborts on 24 of 24 seeds at III, 14 of 24 at V and 2 of 24 at I; with aborts in signing, 22 of 24 at V and 7 of 24 at I give no signature. | model `REPORT.md`, "Exporter aborts" |
| C10 | Verification takes about 23 / 67 / 150 ms on the x86_64 development machine and 167 / 449 / 1023 ms on a Raspberry Pi 4, unoptimized. The Pi figure is for the unsalted verifier. | Ran | `docs/benchmarks/`, `docs/comparison/e4` | `prism-bench` |
| C11 | Honest signatures are accepted: 44,000 fresh level I signatures over 13 keys, 4,000 at III and 2,000 at V on one key, each at the first candidate whose degree passes Baillie–PSW (the first prime index unsalted, the first prime candidate salted). In the single-key runs, reaching it took about 91, 145 and 187 hashes on average unsalted, and 92, 141 and 180 salted. | Ran | `docs/comparison/e6` | `scripts/run_comparison.sh` |
| C12 | No published implementation or timing of the §5 variant existed before this one. | Open | Impression only. Needs a recorded prior-art search. | — |

## Departures from the paper

| # | Paper | Here | Why | Decided |
| --- | --- | --- | --- | --- |
| D1 | Verifier repeats the hash-to-prime search. | The search variable is carried: first an index, now a salt. | Constant verifier cost; then collision replay. | 11-07, 11-12 |
| D2 | Chain length equals the torsion exponent `e`. | Chain length `e′ = e − 2`; signature points keep order `2^e`. | The two theta structures cannot otherwise be identified. | 11-09, 11-10 |
| D3 | Step 8 on a basis of `E_vk[p+1]`. | Step 8 on `E_vk[2^e]`, both components, three points. | Sufficient once the chain is two short; removes a search and a failure mode. | 11-10 |
| D4 | §5 verifier described without pairings. | One Weil pairing identity before the chains. | A non-isotropic kernel does not stop a chain. | 11-11 |
| D5 | No canonical form for `E_sig`. | Least Montgomery coefficient of the class, plus a sign rule. | Strong unforgeability needs one encoding per signature. | 11-10 |
| D6 | Identification of the midpoint structures not specified. | Dual structures from closed-form matrices. | Had to be supplied. | 11-10, 11-11 |
| D7 | Primality test unspecified. | Deterministic Baillie–PSW on one degree. | A consensus verifier cannot use random bases. Open, see O1. | — |
| D8 | Salt uniformly random. | Salt derived from seed, public key, message, counter and caller randomness. | No dependence on the random number generator; auditable when the randomness is zero. | 11-12 |
| D9 | Public key with basis hints (C reference). | Public key is the Montgomery coefficient alone. | Smaller key; the basis is derived. | 11-08 |
| D10 | — | The starting curve `A = 0` is refused as a key. | Its endomorphism ring is public. | 11-08 |
| D11 | Reference key generation as shipped. | Exporter build rejects a non-positive Cornacchia cofactor. | C9. | 11-11 |

## Open obligations

| # | Obligation | Blocks |
| --- | --- | --- |
| O1 | Primality under a signer who chooses message and salt (ePrint 2026/906). | Publication bar 2 |
| O2 | Written proof of A1: acceptance implies a degree-`q` isogeny with the stated action. | Publication bar 3 |
| O3 | Written proof of A3: one byte string per accepted isogeny class. | Publication bar 3 |
| O4 | Distribution of the signer's responses and of its failures; the assumption named in `SECURITY_ARGUMENT.md` §6. | Publication bar 3 |
| O5 | A signer with no C at run time. | Publication bar 1 |
| O6 | Side-channel and secret-handling status of the signer. | Publication bar 4 |
| O7 | Optimized field arithmetic; Pi 4 timing of the salted verifier. | Publication bar 5 |
| O8 | Independent review. | Publication bar 3 |
| O9 | Prior-art search (C12). | Publication bar 7 |
| O10 | Upstream report of C9; follow-up to the authors. | — |
| O11 | A signer can make one signature valid for two messages of its own choosing at the collision cost. Decide whether Shekyl's use needs this closed. | Integration |
