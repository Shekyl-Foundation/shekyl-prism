# Security argument for the merged research protocol

Status: REVIEW DRAFT, 2026-11-12. This is a code-matched conditional
argument, not independent approval or an established SUF-CMA theorem.
The implementation under review is PR #13's merge commit
`d0187579d41d2de275f95fc6fecdf99c272ca324`. No production behavior or
parameters are changed by this evaluation. Source locations below refer
to that commit, including the native exporter and its build script.

## 1. Review conclusion and claim boundary

The existing acceptance argument can establish a prime-degree isogeny
**conditional on correct polarized theta maps and genuine primality**.
It does not by itself establish strong unforgeability. A conditional
SUF-CMA argument additionally needs unique encoding of each isogeny,
a chosen-degree isogeny hardness assumption matching the actual key and
signing distributions, and resistance to collisions in the carried-index
challenge. These conditions are separated below; none is marked reviewed
merely because fixtures pass.

In particular, using the paper's field rows does not import its security
levels. The current protocol is unsalted, has fewer challenge bits, and
checks the carried index rather than recomputing a hash-to-prime search.
There is a concrete collision-replay attack family (§5). Its cost depends
on the density of degrees for which signing succeeds; the existing
measurements do not determine that density over the entire domain.

## 2. Exact object and attacker game

Fix one field row and one trusted, pinned backend build. The key generator
is the implementation's 48-byte uniform master seed, the domain-separated
`keygen` derivation, and native key generation. It is **not assumed** to
sample a uniformly random supersingular curve. Public verification of an
arbitrary well-formed coefficient does not certify an unknown endomorphism
ring; SUF-CMA concerns honestly generated keys.

For `r=e−2`, put `N=2^e`, `D=2^(2r−1)` and

`S = {s odd : 0 < s < 2^(r−1)}`, `|S|=M=2^(r−2)=2^(e−4)`.

The actual hash input is the unambiguous concatenation
`pk_bytes || message || LE16(j)`, under the cSHAKE customization
`shekyl-prism-challenge-v1`. The projection forces oddness and masks high
bits. The candidate degree is `q(s)=D−s²`. The verifier accepts BPSW
probable primes, not certified primes. For positive `s` in `S`, `q(s)` is
injective, lies strictly between `D/2` and `D`, and is `7 mod 8`.

Signing scans `j=0,…,65535`, visits at most 16 BPSW-passing candidates,
and returns the first normalized response accepted by Rust. Native/I/O
or malformed-output errors terminate the call; only a well-decoded
response that fails final verification causes the loop to continue.
There is no fresh-randomness retry. The response stream depends on the
master seed, level, label and **scalar**, not the message or index.
Thus, for a fixed backend and successful deterministic execution, the
response prefix for a given scalar repeats. The index appended to it
can differ. Backend-version text alone is not a cryptographic build pin.

In the single-key SUF-CMA game, the attacker knows the public key, adaptively
requests signatures on arbitrary byte messages, observes responses and
errors, and outputs `(m*,sig*)`. It wins if public verification accepts
and that exact pair was not returned by the signing oracle. A new signature
on a previously signed message counts. Errors do not enter the successful
pair set, but their leakage is part of the transcript.

This argument first considers classical attackers and an idealized,
deterministic, fault-free backend with the implemented error semantics.
Timing, local secret access, faults, concurrency, adaptive key selection,
multi-user losses, quantum oracle access and wallet derivations need
separate arguments. A classical ROM argument is not a QROM proof.

## 3. Code-to-claim map

Paths and line ranges are at the audit pin above. Read the named function
as well as its comment; the comment is not the evidence.

| Claim | Code at audit pin | Disposition |
| --- | --- | --- |
| Fields, exponents, byte sizes | `prism-field/src/lib.rs:270–375` | Integer audit reproducible with `python3 scripts/security_parameters.py`; not a modulus primality proof |
| Hash domain, projection and carried index | `prism-verify/src/challenge.rs:18–110`; `prism-verify/src/lib.rs:109–144`, `checked` | Exact interface differs from a total hash-to-prime function |
| Degree arithmetic and BPSW | `prism-verify/src/prime.rs:610–686` | Adversarial primality remains an assumption |
| Key identity and canonical bytes | `prism-verify/src/identity.rs:30–77`; `prism-verify/src/signature.rs:60–136` | Unique field bytes; raw key bytes bind the message context |
| Generic response models and joint sign | `prism-verify/src/model.rs:12–55`; `signature.rs:110–126` | Conditional map-encoding uniqueness; see §4 |
| On-curve, full torsion, span, pairing | `prism-verify/src/relation.rs:36–151` | Necessary preconditions for the algebraic lemma |
| Adapted bases and two chain lengths | `prism-verify/src/accept.rs:76–149`; `prism-isogeny/src/modular.rs`, `challenge_matrices` | Polarized-chain premise still open |
| Codomain identification and all images | `prism-verify/src/accept.rs:152–184`; `prism-isogeny/src/dual_chain.rs:231–310` | Projective null equality must supply the required polarized identification |
| Determinism, bound, errors, release check | `prism-signer/src/lib.rs:123–226` | Read together with native executable and RNG selection |
| Native key/response sampler and RNG | `scripts/c/export_s5_vectors.c`, `keygen`, `sign`, secure `randombytes`; `scripts/export_s5_vectors.sh`, `backend` | Distribution and side channels are not established by seed derivation |

The native source dependency pins are `PRISM_v2` at
`87fad7da5f12f79112521677f4f5530672afe9ba` and `Theta_dim4` at
`4770b24a61e6aa15da4cac1b9d82fd7b9a05f459`. Local patches, compiler,
arithmetic backend and RNG build mode also belong to the reviewed build.

The public degree helper checks positive odd s and `s²<D`, which is a
wider domain than the challenge projection. The verifier's hash derivation
supplies the narrower bound; the helper alone is not a challenge-domain
validator. The previous helper comment asserted a narrower rejection than
the code performs; this evaluation corrects the comment without changing
the function or verifier behavior.

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

## 5. Collisions and a concrete replay strategy

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
estimate for it. Backend acceptance further changes rho. Neither density
is measured or bounded here. The paper's §5.1 uses conjectural
prime-plus-square counting; its conclusion cannot be substituted for a
calculation at these modified parameters.

A same-message/different-index scalar collision can also reuse a response
prefix by changing the index bytes, which SUF-CMA must cover. The 16-bit
index allows at most 65536 values per message; it supplies no independent
challenge entropy and is not a salt. A single-target second-preimage
attack and an offline chosen-message birthday attack have different costs.

## 6. Conditional SUF-CMA decomposition

Define an auxiliary algebraic oracle experiment, explicitly stronger than
plain one-shot isogeny hardness. It samples the **actual key distribution**
and exposes a cached deterministic response oracle on any scalar in S,
with the actual native sampler, normalization and error behavior. It has
no public message hash and no index search. For successful prime-degree
outputs, record the isogeny class represented by each response. The
attacker's goal is an efficient representation of a prime-degree isogeny
of degree `D−s²` from the key that is different from every recorded class;
the degree may have appeared before. Failed attempts and partial public
outputs must be simulated as well if the API exposes them.

Call this **the implementation-distribution chosen-degree assumption**.
It is an explicit unproved assumption, not a reduction to EndRing, not an
established consequence of uniform SPEDIO² hardness, and not a claim of
uniform signature sampling. It removes the message hash from the problem
but retains the hardest distributional question in the signer.

Under A1–A3, consider a winning SUF forgery. If its represented isogeny
class is new, it wins the auxiliary experiment: signing queries can be
simulated by scanning public hashes and invoking the cached response
oracle with the same 16-candidate and index bounds. This simulation uses
the implementation's failures rather than silently returning an ideal
uniform response. The reduction may make up to 16 algebraic queries per
signing call and up to 65536 challenge queries per call.

Otherwise its class appeared in a successful oracle response at scalar s.
Degree injectivity forces the same scalar in the forgery. The recorded
response might have arisen at a rejected candidate or at a response whose
message/index differs from the final returned one; include those candidate
queries in the collision transcript. By A3 the map fixes the response
prefix. If the message and index input are also the same as that candidate,
the deterministic simulation would accept that same candidate and return
that exact pair. It is then not a new SUF pair. In every remaining case,
two distinct hash inputs have the same projected scalar. Thus the success
event is contained in **new algebraic class OR scalar collision**, apart
from violations of A1–A3 and the ideal execution model.

For classical ideal-hash queries, collision probability is at most
`binom(Q,2)/M`, with Q including adversary queries, signing search queries
and the final verification query. The bound intentionally counts all
scalar collisions, without a prime-density improvement. A formal theorem
must specify resources and account for any sampler/KDF hybrid losses and
algebraic-oracle advantage. This event decomposition supplies no numerical
bound on that advantage and no QROM result.

Accepting a nonminimal index is consequently not by itself a demonstrated
forgery. It requires covering more hash queries and new algebraic outputs
in the proof. Determinism at the message level alone is insufficient;
determinism for a repeated degree is the relevant simulator property.

## 7. Relation to published results

[Basso et al., ePrint 2026/443](https://eprint.iacr.org/2026/443), §5,
defines prime-plus-square degrees and a strong-isogeny representation.
Its §5.1 introduces stronger oracle problems (Problems 4 and 5), including
new isogenies of a previously disclosed degree; the ROM discussion requires
determinism for a given degree. This motivates §6 but does not establish
its actual-distribution assumption or adapted-verifier premises. The
salted construction and its parameter choices are separate sections.

[Nakagawa and Onuki, ePrint 2025/1602](https://eprint.iacr.org/2025/1602)
analyzes small-extension torsion attacks on PRISM-id and expressly
distinguishes the signature setting. It does not certify our structured
challenge distribution or arbitrary-index interface. An adversary can
grind hashes for favorable degrees; a small sample failing to find one
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

This author pass read the challenge, parameter, response, model, identity,
relation, acceptance, dual-chain, scalar arithmetic, Rust signer and native
exporter surfaces listed in §3, and the relevant published security
sections. It found no alternate field encoding in those decoder paths or
message/index framing ambiguity at a fixed row. These are source-reading
observations, not independent audit conclusions. The numerical script
checks exact integer bounds, not primality or map correctness.

This pass did not audit every field operation, prove theta formulas on
adversarial loci, establish rational model-transport completeness, prove
sampler distributions, evaluate quaternion timing/memory access, run a
fresh Sage differential, or calculate best classical/quantum attack costs.
Previously passing CI and measurements belong to PR #13; this document
does not report a new execution of that corpus.

| Obligation | Required review result | Current status |
| --- | --- | --- |
| A1 | Correct maps, polarizations, dual identification and evaluation on every accepting input | OPEN; algebraic reviewer needed |
| A2 | Justified adversarial primality policy, or protocol change with certified primes | OPEN; BPSW only |
| A3 | Complete rational transports and unique bytes per isogeny class | OPEN; conditional lemma only |
| Key/sampler distribution | Bound distance from the published oracle/key distributions, or justify the exact-distribution assumption | OPEN; not implied by SHAKE |
| Challenge and concrete costs | Bound usable density, collision/grinding work, memory, and other classical/quantum attacks for these rows | OPEN; §5 exposes the collision constraint |
| Formal reduction | Resource-bounded proof, including errors, retries and QROM if claimed | OPEN; classical event decomposition only |
| Implementation security | Review native secrets, timing, allocations, abnormal exits and compiler/backend pin | OPEN; no constant-time claim |
| Independent approval | Named reviewer, reviewed commit, findings and signed-off scope | PENDING; no review approval recorded |

No external reviewer has been contacted by this work. Human review of the
separate security PR is required before changing this status. Protocol
changes, if review requires them, need their own implementation PR and
new vectors; this document does not silently retrofit salt or parameters.
