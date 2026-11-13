# Security argument for the salted research protocol

Status: REVIEW DRAFT, 2026-11-13. This is a code-matched conditional
argument, not independent approval or an established SUF-CMA theorem.
The implementation under review is the salted protocol of PR #15 at commit
`ef2fba65e576a61576164f9d7b44ea02246172b0`. Source locations below name
files and functions at that commit. The unsalted protocol this replaces
is at tag `archive/unsalted-index-2026-11-12` (commit `d018757`); its
analysis is Appendix A.

## 1. Review conclusion and claim boundary

The existing acceptance argument can establish a prime-degree isogeny
**conditional on correct polarized theta maps and genuine primality**.
It does not by itself establish strong unforgeability. A conditional
SUF-CMA argument additionally needs unique encoding of each isogeny,
a chosen-degree isogeny hardness assumption matching the actual key and
signing distributions, a pseudorandom salt schedule, and resistance to
second preimages onto a signed challenge. These conditions are separated
below; none is marked reviewed merely because fixtures pass.

The challenge now binds a carried salt. A third party can no longer
prepare two messages with one challenge before the signer acts, because
it does not know the salt the signer will use. What it can still do is
look for a second hash input that lands on a scalar the signer already
signed (§5). At the full width that costs about `2^b / N_sign` hash
queries for `N_sign` signatures, against about `sqrt(2^b / rho)` for the
unsalted collision replay. [SALT_COMPARISON.md](SALT_COMPARISON.md) holds
the measurements, including both attacks carried out against the real
signer and verifier at reduced challenge widths.

Using the paper's field rows still does not import its security levels.
The salt changes the hash-side term only. The polarized-map, primality
and encoding obligations A1 to A3 (§4), the isogeny assumption (§6) and
the signer's side channels are exactly as open as before.

## 2. Exact object and attacker game

Fix one field row and one trusted, pinned backend build. The key generator
is the implementation's 48-byte uniform master seed, the domain-separated
`keygen` derivation, and native key generation. It is **not assumed** to
sample a uniformly random supersingular curve. Public verification of an
arbitrary well-formed coefficient does not certify an unknown endomorphism
ring; SUF-CMA concerns honestly generated keys.

For `r=e−2`, put `N=2^e`, `D=2^(2r−1)` and

`S = {s odd : 0 < s < 2^(r−1)}`, `|S|=M=2^(r−2)=2^(e−4)`.

Write `b=e−4` for the free bits of `s` (244, 372, 496) and `k=8·SALT_LEN`
for the salt bits (256, 384, 512; `k=2·lambda`).

The hash input is the concatenation `pk_bytes || salt || message` under
the cSHAKE256 customization `shekyl-prism-challenge-v2`. Both leading
fields have a fixed length for the row, so the framing is unambiguous.
The projection forces oddness and masks high bits. The candidate degree
is `q(s)=D−s²`. The verifier accepts BPSW probable primes, not certified
primes. For positive `s` in `S`, `q(s)` is injective, lies strictly
between `D/2` and `D`, and is `7 mod 8`.

A signature is five canonical field elements and the salt. Every byte
string of length `SALT_LEN` is an admissible salt. There is no canonical
salt form to check: two signatures that differ only in the salt hash to
different scalars except with the collision probability of §5, so the
binding of the salt comes from the hash and not from its encoding. The
verifier derives one scalar from the carried salt, tests that one degree
and never searches.

Signing takes a 32-byte caller input `R` (all zeros in the research API
and the committed vectors). With `salt_key = stream_seed("salt", "")`,

`salt_i = cSHAKE256("shekyl-prism-salt-v1"; salt_key || pk_bytes || R || LE64(i) || message)[..SALT_LEN]`

for `i = 0,…,65535`. The signer visits at most 16 counters whose degree
passes BPSW. At each it asks the native backend for a response, with a
stream keyed on the master seed, level, label and **scalar** only, then
normalizes the response and runs the Rust verifier on the exact bytes.
A backend error, a malformed response or a refused response advances
the counter. Exhaustion of either bound returns `Exhausted`; setup and
key-generation errors are typed errors. The counter is never output.
The signer draws no randomness of its own. For fixed key, backend, `R`
and message the output repeats, and for a fixed scalar the response
prefix repeats whatever the message or salt. Backend-version text alone
is not a cryptographic build pin.

In the single-key SUF-CMA game, the attacker knows the public key,
adaptively requests signatures on arbitrary byte messages, observes
responses and errors, and outputs `(m*,sig*)`. It wins if public
verification accepts and that exact pair was not returned by the signing
oracle. A new signature on a previously signed message counts, including
one that differs only in its salt. The signing oracle of the game uses
the signer's own `R` policy; an attacker who chooses `R` is a different
game (§5, "What the salt does not change"). Errors do not enter the
successful pair set, but their leakage is part of the transcript.

This argument first considers classical attackers and an idealized,
deterministic, fault-free backend with the implemented error semantics.
Timing, local secret access, faults, concurrency, adaptive key selection,
multi-user losses, quantum oracle access and wallet derivations need
separate arguments. A classical ROM argument is not a QROM proof.

## 3. Code-to-claim map

Read the named function as well as its comment; the comment is not the
evidence.

| Claim | Code at the pin | Disposition |
| --- | --- | --- |
| Fields, exponents, byte and salt sizes | `prism-field/src/lib.rs`, `nist_level!` and `Salt` | Integer audit reproducible with `python3 scripts/security_parameters.py`; not a modulus primality proof |
| Hash domain, framing and projection | `prism-verify/src/challenge.rs`, `ChallengeScalar::derive` and `with_width`; `prism-verify/src/lib.rs`, `checked` | One hash per verification; no search; any salt of the row length is admissible |
| Reduced-width entry points | `challenge.rs`, `derive_width`, `research_bytes`; `prism-verify/src/research.rs` | Compiled only under `research-reduced-width`; `scripts/ci/check_research_boundary.py` builds a consumer to show their absence by default |
| Degree arithmetic and BPSW | `prism-verify/src/prime.rs`, `challenge_degree`, `degree_is_prime`, `is_prime` | Adversarial primality remains an assumption |
| Key identity and canonical bytes | `prism-verify/src/identity.rs`, `KeyIdentity::decode`; `prism-verify/src/signature.rs`, `Response::decode` | Unique field bytes; raw key bytes bind the message context |
| Generic response models and joint sign | `prism-verify/src/model.rs`, `check`; `signature.rs`, `canonical_sign` | Conditional map-encoding uniqueness; see §4 |
| On-curve, full torsion, span, pairing | `prism-verify/src/relation.rs`, `relate_points`, `pairing_holds` | Necessary preconditions for the algebraic lemma |
| Adapted bases and two chain lengths | `prism-verify/src/accept.rs`; `prism-isogeny/src/modular.rs`, `challenge_matrices` | Polarized-chain premise still open |
| Codomain identification and all images | `prism-verify/src/accept.rs`; `prism-isogeny/src/dual_chain.rs`, `matches_dual`, `dual_eval` | Projective null equality must supply the required polarized identification |
| Salt schedule, retry policy, bounds, release check | `prism-signer/src/lib.rs`, `sign_level`, `stream_seed` | Read together with native executable and RNG selection |
| Native key/response sampler and RNG | `scripts/c/export_s5_vectors.c`, `keygen`, `sign`, secure `randombytes`; `scripts/export_s5_vectors.sh`, `backend` | Distribution and side channels are not established by seed derivation |

The native source dependency pins are `PRISM_v2` at
`87fad7da5f12f79112521677f4f5530672afe9ba` and `Theta_dim4` at
`4770b24a61e6aa15da4cac1b9d82fd7b9a05f459`. Local patches, compiler,
arithmetic backend and RNG build mode also belong to the reviewed build.

The public degree helper checks positive odd s and `s²<D`, which is a
wider domain than the challenge projection. The verifier's hash derivation
supplies the narrower bound; the helper alone is not a challenge-domain
validator.

## 4. Acceptance and representation lemmas

The detailed derivation is in [ACCEPTANCE_ARGUMENT.md](ACCEPTANCE_ARGUMENT.md).
Its obligations can be stated precisely:

**A1: polarized computation.** For every adversarial input that reaches
acceptance, the chains represent polarized separable maps of multipliers
`2^r` and `2^(r−1)`. The projective Hadamard identification used by
`matches_dual` is the compatible polarization-preserving isomorphism, and
`dual_eval` evaluates its intended dual. This is stronger than agreement
on honest traces. In particular, exceptional intermediate loci and
nonzero-but-invalid theta coordinates need an algebraic analysis.

Under A1, their composite `F` has `F†F=[D]`. Its first column `(α,β)`
satisfies `deg α + deg β = D`. Full rational order-`N` bases and the
three comparisons on `P,Q,P−Q` force one common sign in each component.
Hence `α` restricts to `±[s]` on `E_vk[N]`. The degree norm bound
`deg(α∓[s]) ≤ (sqrt(D)+s)² < N²` forces `α=±[s]` globally, and
therefore `deg β=D−s²`. The two component signs need not coincide;
negating β is absorbed by the signature's joint sign normalization.

The torsion checks also establish the maximal supersingular curve class:
`N²>4p`, while `(p+1)²` is a multiple of `N²` and is the upper endpoint
of the Hasse interval. It is consequently the only possible rational
point count divisible by `N²`. This uses actual independent rational
points and nonsingularity, not just a basis search result.

**A2: prime degree.** The accepted `q` is genuinely prime. The norm argument
still derives degree `q` if it is composite, but the prime-degree hardness
assumption and resistance to small-factor modifications no longer follow.
There is no justified numerical failure bound for adversarial BPSW inputs
in this review. Random Miller–Rabin error estimates do not apply to this
fixed deterministic predicate.

**A3: unique encoding of one map.** On the accepted nonexceptional response
class, rational Montgomery transports enumerate the complete relevant
isomorphism class. The least coefficient and joint sign yield exactly one
prefix per isogeny up to codomain isomorphism. For a fixed codomain, two
degree-q maps agreeing on full `N`-torsion agree globally, since
`deg(β₁−β₂)≤4q<N²`. This implication is one-way: the degree bound does
**not** say every degree-q map from the key is identical. A3 still needs
a proof of rational transports and their completeness, including twists
and the relative sign chosen by public-basis recovery.

The executable parameter audit checks the integer inequalities at all
three rows. It proves neither A1, A2 nor A3. The degree interval also has
`q>p`, so an actually prime q cannot equal the characteristic.

## 5. Second preimages, and what the salt changes

A response is bound to its scalar, not to its message: the same prefix
verifies under any `(salt, message)` whose hash is that scalar. Every
hash-side attack is therefore a way of producing two different hash
inputs with one prime-degree scalar, one of which the signer signs.

**Before the signer acts.** In the unsalted protocol an attacker fixed
the public key, collided two messages offline, had one signed and reused
the bytes for the other (Appendix A). Here the hash input contains the
salt, and the signer derives it from a secret key. An attacker who
prepares a collision under a salt of its own choosing holds a collision
for a salt the signer will not use. For the preparation to pay, the
attacker must have queried the hash at the signer's salt beforehand. With
`N_H` hash queries and `N_c` signer candidates, and salts that are
pseudorandom among the roughly `rho·2^k` salts giving a prime degree,
that happens with probability at most about `N_H·N_c/(rho·2^k)`.

**After the signer acts.** Given signatures at scalars `s_1,…,s_n`, the
attacker searches `(salt*, m*)` hashing onto any of them. The forged
salt is free, so nothing restricts the search except the digest space.
With `N_H` queries the success probability is about `N_H·n/2^b`, that
is `2^b/n` queries for constant success. This is a multi-target second
preimage, with cost linear in the digest space, where the unsalted replay
is a birthday search with cost in its square root.

| Row | Free bits `b` | Measured density `rho` | Unsalted collision replay | Salted, one signed target | Salted, `2^64` signed targets |
| --- | ---: | ---: | ---: | ---: | ---: |
| I | 244 | 1.055% | `2^125.3` | `2^244` | `2^180` |
| III | 372 | 0.695% | `2^189.6` | `2^372` | `2^308` |
| V | 496 | 0.584% | `2^251.7` | `2^496` | `2^432` |

Costs are hash queries for constant success, without constant factors,
memory or the cost of one hash. They are not AES-equivalent security
levels and not a NIST category claim. **Derived** from the density
**Ran** as experiment E1 of [SALT_COMPARISON.md](SALT_COMPARISON.md):
`10^5` challenges per row under each hash version, at the verifier's
width, with the production projection and Baillie–PSW test. The density
is of BPSW-passing degrees; identifying them with primes uses A2, and
identifying them with signable degrees assumes the backend succeeds on
them, which E6 of the same document measures.

**Ran, at reduced width only.** Experiment E3 carries out the unsalted
replay, the same preparation against the salted signer, and the second
preimage, against the real native signer and the real algebraic verifier
at row I, with the challenge projected to 16 to 32 bits and nothing else
changed. Query counts grow with half the width for the unsalted replay
and with the full width for the salted second preimage. The full-width
figures in the table are an extrapolation of those slopes and of the
random-oracle estimate. No full-width forgery was attempted or measured.

### The counter is not public

The archived signature carried its index, so anyone could count the
earlier indices with a prime degree, each of which the signer had tried
and discarded (Appendix A). The salted signature carries `salt_i` and not
`i`. Without `salt_key` the salts of the earlier counters cannot be
derived, so neither the number of composite candidates nor the number of
discarded prime candidates can be computed from the public key, message
and signature. This is a statement about the bytes of the signature.
Signing time still grows with the number of candidates and backend calls,
and no timing claim is made.

### A failed candidate no longer ends the call

In the archived signer a backend error at the first prime index ended the
call, and the same public search order and scalar-keyed stream made it
fail again on every retry. The salted signer advances the counter on a
backend error, a malformed response or a refused response, and the next
counter gives an unrelated scalar. A message is unsignable only if 16
prime candidates in a row fail, or no prime candidate is accepted in
65,536 counters. Experiment E6 signs the same messages with both loops
and reports how often either event occurred.

### What the salt does not change

- **The signer can still collide its own messages.** Whoever chooses the
  salt can search two messages with one prime-degree scalar under it, at
  the unsalted collision cost `sqrt(2^b/rho)`, and issue one signature
  valid for both. This is the paper's Remark 13: salt-PRISM has `a/2`-bit
  security against message-bound-signature attacks. Signer-side message
  binding is outside this protocol.
- **A predictable salt returns the unsalted cost.** The argument above
  uses the salt's unpredictability before signing. If `salt_key` leaks,
  or a signer derives salts from public data, or an attacker can make
  the signer use a salt it chose, the attacker prepares its collision
  under that salt and the replay of Appendix A applies unchanged.
  Zero caller randomness is not this case: the salt is still a
  secret-keyed function of the message, and learning it requires the
  signature it belongs to.
- **Caller randomness is a hedge, not a proof.** With fresh `R` the salt
  stays unpredictable if either `salt_key` or `R` is unknown to the
  attacker, under a PRF assumption on cSHAKE256 keyed by `salt_key` and
  on its use keyed by `R`. Nothing here proves resistance to faults or
  side channels, and an `R` chosen by a third party is not covered.
- **A1 to A3 are untouched.** The polarized-map, primality and encoding
  obligations of §4 do not involve the challenge hash.

## 6. Conditional SUF-CMA decomposition

Define an auxiliary algebraic oracle experiment, explicitly stronger than
plain one-shot isogeny hardness. It samples the **actual key distribution**
and exposes a cached deterministic response oracle on any scalar in S,
with the actual native sampler, normalization and error behavior. It has
no public message hash, no salt and no counter. For successful
prime-degree outputs, record the isogeny class represented by each
response. The attacker's goal is an efficient representation of a
prime-degree isogeny of degree `D−s²` from the key that is different from
every recorded class; the degree may have appeared before. Failed
attempts and partial public outputs must be simulated as well if the API
exposes them.

Call this **the implementation-distribution chosen-degree assumption**.
It is an explicit unproved assumption, not a reduction to EndRing, not an
established consequence of uniform SPEDIO² hardness, and not a claim of
uniform signature sampling. It removes the message hash from the problem
but retains the hardest distributional question in the signer.

Under A1–A3, consider a winning SUF forgery `(m*, prefix* || salt*)`. If
its represented isogeny class is new, it wins the auxiliary experiment:
a signing query is simulated by running the salt schedule, hashing, and
invoking the cached response oracle at each prime candidate, with the
same 16-candidate and 65,536-counter bounds and the implementation's own
failures. The simulator needs `salt_key`; a proof replaces the schedule
by uniform salts at the cost of the PRF advantage. The reduction makes
up to 16 algebraic queries and 65,536 hash queries per signing call.

Otherwise its class appeared in an oracle response at scalar `s`, for
some signer candidate `(salt_i, m_i)`. Degree injectivity forces the same
scalar in the forgery, and by A3 the class fixes the response prefix. If
`(salt*, m*) = (salt_i, m_i)`, the forgery is the exact bytes the signer
verified at that candidate: either it was released, and is not a new
pair, or it was refused, and is refused again. In every remaining case
`(salt*, m*)` and `(salt_i, m_i)` are distinct hash inputs with one
scalar, and one of them was chosen by the signer. The success event is
therefore contained in

**new algebraic class, OR a second preimage onto a signed scalar,**

apart from violations of A1–A3 and the ideal execution model. "Signed"
includes candidates the signer visited and discarded; they are never
released, but their scalars are part of the simulation.

### The published bound and its mapping

The second event is the biased multi-target extended target collision
game of [ePrint 2026/443](https://eprint.iacr.org/2026/443) Lemma 1,
with first input the salt (`|X1| = 2^k`), digest space the odd scalars
(`|Y| = 2^b`), accepted subset the scalars of prime degree and
`gamma = rho`. Its classical bound, equation (9), is

`(q_h+1)·q_b/2^b + q_h·q_b/(rho·2^k)`,

with `q_h = N_H` hash queries and `q_b` the number of signer candidates.
Theorem 2 adds the probability that the signer's salt was already
queried (equation (14)) and a factor `N_H` on the isogeny problem, giving
equation (12):

`Adv ≤ N_H·Adv_Problem3 + (N_H+1)·N_sign/2^n + (N_sign+N_H)·N_sign/(gamma·2^(k−1))`.

Divided by the attack cost `N_sign + N_H`, equation (18) leaves
`Adv_Problem3 + N_sign/2^n + N_sign/(gamma·2^(k−1))`. With our sizes,
`n = b`, `gamma = rho` and `N_sign = 2^64`:

| Row | `N_sign/2^b` | `N_sign/(rho·2^(k−1))` | Sum |
| --- | ---: | ---: | ---: |
| I | `2^-180` | `2^-184.4` | `2^-179.93` |
| III | `2^-308` | `2^-311.8` | `2^-307.90` |
| V | `2^-432` | `2^-439.6` | `2^-431.99` |

**Checked against the bundled text.** The brief's derived figure, about
`2^-180` at row I, is equation (18) with these substitutions and holds.
Two remarks on the source. The proof's equation (11) writes the
reprogramming term as `q_b(q_b+q_h)/(gamma·|X2|)` while the lemma's
statement (9) has `q_h·q_b/(gamma·|X1|)`; the salt space is `X1`, and
(12) is consistent with the larger numerator over `2^k`. And the paper
chooses `n = lambda + 64`, so its first term meets `2^-lambda` exactly;
ours is far below it because `b` was fixed by the §5 relation, not by
this bound.

**What the mapping does not carry over.** Each line is an obligation a
formal statement must discharge.

- *Salt distribution.* The theorem's signer samples a uniform salt until
  the digest is prime. Ours takes the first accepted counter of a
  secret-keyed schedule. Replacing the schedule by uniform salts costs a
  PRF advantage for cSHAKE256 keyed by `salt_key`, or by `R` when the
  hedge is what is relied on. Repeated signing of one message with one
  `R` repeats the salt and the signature, which the reprogramming
  argument tolerates but must state.
- *Failure distribution.* The theorem needs the released digest uniform
  over the accepted subset. Ours is uniform over prime-degree scalars
  only if backend errors and refused responses are independent of the
  scalar, or absent. E6 measures their frequency; a count of zero bounds
  the rate statistically and proves nothing about the distribution.
- *Candidate count.* `q_b` is the number of scalars the signer visits,
  up to 16 per call, not the number of signatures.
- *The isogeny problem.* Problem 3 has degrees `q(2^a − q)` for `q`
  uniform among `a`-bit primes, uniform isogenies and a uniformly random
  curve. This protocol uses the §5 relation, degree `D − s²`, the native
  sampler and the implemented key generator. The matching statement is
  the assumption defined above, or the paper's Problem 5 if its
  distributions were shown to hold.
- *EUF, not SUF.* Theorem 2 is an EUF-CMA statement. The sentence after
  its proof (Remark 11) says the argument uses only the density of the
  accepted digest set and that signatures are valid on random accepted
  digests, and that it "can be straightforwardly applied to a salted
  version of the strongly unforgeable variant presented in Section 5".
  That transfers the hash-side term. Strong unforgeability still needs
  one response per isogeny class, which is A3, and the stronger problem
  in which an isogeny of an already-seen degree is a solution.
- *Classical only.* Equations (10) and (13) are the QROM forms; no QROM
  claim is made here.

The table is a bound on one term of a conditional statement. It is not a
security level for these rows.

## 7. Relation to published results

[Basso et al., ePrint 2026/443](https://eprint.iacr.org/2026/443), §5,
defines prime-plus-square degrees and a strong-isogeny representation.
Its §5.1 introduces stronger oracle problems (Problems 4 and 5), including
new isogenies of a previously disclosed degree; the ROM discussion requires
determinism for a given degree. This motivates §6 but does not establish
its actual-distribution assumption or adapted-verifier premises. Its §6
defines salt-PRISM, proves Theorem 2 and chooses `k = 2·lambda` "for
little cost"; §6 above maps that result to this protocol and lists what
does not carry over. The paper's signer samples its salt at random; this
one derives it, with an optional caller input.

[Nakagawa and Onuki, ePrint 2025/1602](https://eprint.iacr.org/2025/1602)
analyzes small-extension torsion attacks on PRISM-id and expressly
distinguishes the signature setting. It does not certify our structured
challenge distribution. An adversary can choose the salt of a forgery and
so grind hashes for favorable degrees; a small sample failing to find one
cannot certify the absence of this avenue.

The bundled SPIDER and SPRITZ papers concern additional constructions;
their results are not imported as security of this signer. The bundled
orientation-algorithm paper affects concrete isogeny cost analysis;
this review has not evaluated its costs at these exact primes. Neither
generic EndRing hardness nor a familiar field size alone establishes a
concrete bit-security figure.

For reproducibility, the bundled 2026/443 PDF SHA-256 is
`fb8cb9c20c5250b8ca9ffab8b1593aed2af350c2744e1e63468c152ee5a013a1`;
the bundled 2025/1602 PDF is
`46b77ac22961c5dbb45fd2d875241965a8fef15f7c3800e57f3798b25c235560`.
The online ePrint records may be revised; quoted section/problem numbers
above refer to the bundled texts, not an unpinned future revision.

## 8. Review record and remaining decisions

Maintainer review of the unsalted argument (`a847368`, with the measured
density and the index and signing-failure findings) was accepted, and the
maintainer then ruled on 2026-11-12 that the carried index is replaced by
a carried salt and the findings published with and without it. That
review is recorded from the maintainer's instructions; it is not a claim
that an external cryptographer discharged A1–A3 or the hardness
assumptions.

This pass read the salted challenge, signature, signer and research
surfaces listed in §3 and the bundled paper's §6.1, and found no framing
ambiguity at a fixed row. These are source-reading observations, not
independent audit conclusions. The numerical script checks exact integer
bounds, not primality or map correctness.

What was executed for this protocol is recorded in
[SALT_COMPARISON.md](SALT_COMPARISON.md) and, for the Sage comparison, in
[SECURITY_REVIEW.md](SECURITY_REVIEW.md). This document reports no
execution of its own.

This pass did not audit every field operation, prove theta formulas on
adversarial loci, establish rational model-transport completeness, prove
sampler distributions, evaluate quaternion timing or memory access, or
calculate best classical or quantum attack costs.

| Obligation | Required review result | Current status |
| --- | --- | --- |
| A1 | Correct maps, polarizations, dual identification and evaluation on every accepting input | OPEN; algebraic reviewer needed |
| A2 | Justified adversarial primality policy, or protocol change with certified primes | OPEN; BPSW only. A forger chooses its salt and can grind degrees |
| A3 | Complete rational transports and unique bytes per isogeny class | OPEN; conditional lemma only |
| Key/sampler distribution | Bound distance from the published oracle/key distributions, or justify the exact-distribution assumption | OPEN; not implied by SHAKE |
| Salt schedule | PRF statement for the keyed cSHAKE256 schedule and the caller-randomness hedge | OPEN; stated as an assumption in §6 |
| Failure distribution | Released scalars uniform over prime-degree scalars, or a bound on the deviation | OPEN; frequency measured, distribution not established |
| Challenge and concrete costs | Second-preimage and signer-collision work, memory, and other classical/quantum attacks for these rows | OPEN; §5 and §6 give the random-oracle estimates |
| Formal reduction | Resource-bounded proof, including candidates, errors and QROM if claimed | OPEN; event decomposition and a mapped published bound only |
| Implementation security | Review native secrets, timing, allocations, abnormal exits and compiler/backend pin | OPEN; no constant-time claim |
| Independent approval | Named cryptographic reviewer, reviewed commit, findings and signed-off scope | PENDING; no independent cryptographic approval recorded |

No external reviewer has been contacted by this work.

## Appendix A. The unsalted protocol

This appendix is the analysis of the protocol at tag
`archive/unsalted-index-2026-11-12` (commit `d018757`), kept because the
comparison is published with and without the salt. Nothing in it
describes the current verifier. There the hash input was
`pk_bytes || message || LE16(j)` under `shekyl-prism-challenge-v1`, the
signature carried the two-byte index `j`, and signing scanned
`j=0,…,65535`, returned the first accepted prime index, and ended the
call on a backend error. The complete argument for that protocol is this
file at the tag.

### A.1 Collisions and a concrete replay strategy

Fix the public key and `j=0`. Search distinct messages whose scalars at
this index collide. When the common scalar has a BPSW-passing degree and
the native signer produces an accepted response at that first index,
request a signature on one message and output the identical signature
for the other. Both verifications execute the same relation. The second
message was never signed, so this breaks even ordinary EUF-CMA.
Deterministic-per-degree signing does not prevent this attack.

Let `G⊆S` contain precisely those scalars for which this first-index
signing attempt succeeds for the fixed key and backend, and write
`rho=|G|/M`. Failed decoding or native calls are excluded from G. An
attacker need not predict success: it can try collision pairs and observe
the signing result. This may cost multiple signing queries. If the
verifier accepted the degree as prime but the signer selected a later
index, that signature is not evidence of this attack's success.

For t distinct message queries under an ideal projected hash, the exact
expected number of unordered collision pairs whose common value is in G is

`E[pairs] = binom(t,2) * |G|/M² = binom(t,2) * rho/M`.

The usual birthday/Poisson estimate gives a constant-success query scale
`Theta(sqrt(M/rho))` for a fixed positive density, with memory and sorting
costs to be counted separately. Testing only colliding candidates avoids
a primality test on every hash query. This is a generic attack estimate,
not a proven lower bound against all attacks, a timing benchmark, or a
claim that arbitrary 491-bit degrees provide 491 challenge bits.

| Row label | Free bits b | Raw birthday exponent b/2 | Nominal lambda | Density needed for `sqrt(M/rho) ≥ 2^lambda` |
| --- | ---: | ---: | ---: | --- |
| I | 244 | 122 | 128 | `rho ≤ 2^-12` |
| III | 372 | 186 | 192 | `rho ≤ 2^-12` |
| V | 496 | 248 | 256 | `rho ≤ 2^-16` |

This comparison omits constant success factors and counts hash queries,
not equivalent AES gates. It is not a NIST category certification or a
proof that the rows fail a specified category. Prime density in this
quadratic family has arithmetic structure; `2/log(q)` is not an established
estimate for it. The measured density below replaces that generic estimate;
native signing acceptance further changes rho and is not measured by the
prime-search driver. The paper's §5.1 conjectural prime-plus-square counting
does not substitute for measurements at these modified parameters.

### A.2 Measured density and the paper's collision criterion

**Ran:** [hosted run 43284546324](https://github.com/Shekyl-Foundation/shekyl-prism/actions/runs/43284546324)
executed the real `sample_e_prime` at the verifier's `e−3` challenge width,
with 1,500 successful first-prime searches per row. Each returned index j
represents `j+1` candidate hashes and exactly one passing degree, giving
the empirical density `rho_prime = 1500/sum(j+1)`. These are 1,500 searches,
not 1,500 total candidate draws. Raw scalar/degree samples, hashes,
source head and the actual tested merge SHA are committed in
[comparison/unsalted-density/record.json](comparison/unsalted-density/record.json)
and [samples.txt](comparison/unsalted-density/samples.txt).

| Row | Searches | Candidate hashes | Measured `rho_prime` | `log2 sqrt(2^b/rho_prime)` | Nominal lambda |
| --- | ---: | ---: | ---: | ---: | ---: |
| I | 1500 | 137730 | 1.08909% | 125.260 | 128 |
| III | 1500 | 208627 | 0.71899% | 189.560 | 192 |
| V | 1500 | 285423 | 0.52554% | 251.786 | 256 |

**Derived from the run:** at these measured densities, all three rows fall
below nominal in the conservative hash-query collision criterion used by
the paper's equation (7). This is a prime-eligible collision estimate, not
a measured full-width forgery. The predicate is BPSW; identifying passing
degrees with primes uses A2. Identifying `rho_prime` with the usable density
rho additionally assumes those degrees produce accepted native responses.
If their success fraction is eta, substitute `rho_prime*eta` in the cost
formula. This run tests no native signing responses and certifies no
whole-domain density or concrete AES-equivalent security level.

For comparison, the paper's §4.4 equation (7) estimates
`t ≈ 2^((a−2)/2)*sqrt(a)` and selects **a=251** for nominal row I
(`log2(t)≈128.49`). It then notes its concrete **a=248** choice
(`log2(t)≈126.98` in that same approximation), crediting hash-evaluation
complexity for the deficit. Our `125.260` estimate instead uses the actual
244-free-bit projection and measured prime-plus-square eligibility; it is
not the paper's a-bit-prime digest domain. Neither a=248 alone nor a
491-bit isogeny degree supplies a conservative 128-bit hash-query claim.

Reproduce the capture at the source head in `record.json` (the sampler
has since moved to `prism-compare`):

```sh
cargo run --release --locked -p prism-verify --bin sample_e_prime -- 1500 > samples.txt
python3 scripts/measure_unsalted_density.py samples.txt --count 1500
```

The sampler uses its fixed key mixer and counter-based messages, with no
random seed or native signer. Its synthetic decoded keys exercise the
challenge hash, not the honest-key generation distribution. To reproduce
the table from committed raw data without a build:

```sh
python3 scripts/measure_unsalted_density.py docs/comparison/unsalted-density/samples.txt --count 1500
```

A same-message/different-index scalar collision can also reuse a response
prefix by changing the index bytes, which SUF-CMA must cover. The 16-bit
index allows at most 65536 values per message; it supplies no independent
challenge entropy and is not a salt. A single-target second-preimage
attack and an offline chosen-message birthday attack have different costs.

### A.3 Public index disclosure and deterministic signing failures

For a returned signature carrying index j, anyone can derive every earlier
scalar using the public key, message and indices `0,…,j−1`, and run the
same BPSW predicate. The number of earlier passing degrees is consequently
public. In the implemented Rust signer, each such earlier candidate
produced a decoded response that failed final verification; otherwise
signing would already have returned it. This is a count of secret-sampler
dependent skipped candidates, not merely a count of public composite
degrees. Native or malformed-output errors instead terminate the call,
so the index does not count skipped backend errors in this implementation.
Fixture exporters that choose the first prime index need not exhibit any
such skips. Experiment E5 of [SALT_COMPARISON.md](SALT_COMPARISON.md)
counts them on the archived corpus.

For a fixed key, message and backend build, a reproducible deterministic
native failure occurs again on retry: the public search order and the
key/scalar-derived secret streams are unchanged. A message that hits this
failure before a successful return cannot be signed by the current API
under that key/build. Deterministic exhaustion of the candidate or index
bound has the same availability consequence. A transient I/O failure or
an externally killed process is not evidence of permanent unsignability.
No concrete unsignable message is claimed by this source-level argument;
experiment E6 of the comparison reports the search and its result.
