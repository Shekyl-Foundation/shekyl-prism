# Research verifier specification

Status: salted executable prototype, 2026-11-12. This is a local instantiation of
PRISM's prime-degree §5 relation with a salted challenge, not an adopted standard or a reviewed
SUF-CMA security claim. See [security review](SECURITY_REVIEW.md).
The [code-matched security argument draft](SECURITY_ARGUMENT.md) defines
the exact attacker game, parameter audit and conditional proof obligations.
The [earlier design notes](archive/VERIFIER_SPEC_before_acceptance_2026-11-12.md)
preserve the derivations, investigations and questions that led here.

## Parameters and encoding

| Parameter | `p248` | `p324` | `p376` | `p500` | `p664` |
| --- | --- | --- | --- | --- | --- |
| `p = f·2^e − 1` | `5·2^248−1` | `3·2^324−1` | `65·2^376−1` | `27·2^500−1` | `17·2^664−1` |
| Rational torsion exponent `e` | 248 | 324 | 376 | 500 | 664 |
| Long chain `e′ = e−2` | 246 | 322 | 374 | 498 | 662 |
| Short chain `e′−1` | 245 | 321 | 373 | 497 | 661 |
| Challenge width `e′−1` | 245 | 321 | 373 | 497 | 661 |
| Free bits in odd `s` | 244 | 320 | 372 | 496 | 660 |
| Degree `q` bit length | 491 | 643 | 747 | 995 | 1,323 |
| Width of the integers `q` is tested in, bits | 1,024 | 1,024 | 1,024 | 1,024 | 1,344 |
| Public-key bytes | 64 | 82 | 96 | 128 | 168 |
| Salt bytes (`e` rounded up to a multiple of 128 bits) | 32 | 48 | 48 | 64 | 96 |
| Search bound (the salt's bit length) | 256 | 384 | 384 | 512 | 768 |
| Signature bytes | 352 | 458 | 528 | 704 | 936 |

`p324` and `p664` are in the code on the verifier side only. No signer
exists for either, so no signature has been accepted at them; see
[PARAMETER_SETS.md](PARAMETER_SETS.md).

The primality test of `q` runs on fixed-width integers whose width the
set names. A set needs `2e − 4` bits: `q < 2^(2e−5)`, and the test forms
that power of two and, in its Lucas half, `q + 1`, each of which can have
its top bit at index `2e − 5`. The four sets up to `p500` keep the 1,024
bits they were built with; `p664` needs 1,324 and has 1,344, the next
multiple of 64. The verdict on an integer does not depend on the width
it is held in.

A parameter set is a prime and is named by it
([PARAMETER_SETS.md](PARAMETER_SETS.md)); the paper's Table 2 calls
`p248`, `p376` and `p500` NIST I, III and V, which does not certify a security level of this
modified salted protocol. `CHAIN_STEPS` is `e′`; `E` is
always the rational torsion exponent. `CHALLENGE_BITS` is the actual
`e−3` width, including the forced odd bit. The historical `e−1` constant
had the same byte width but overstated the number of challenge bits.

A public key is the canonical little-endian fp2 encoding of the Montgomery
coefficient `A`, for `y²=x³+Ax²+x`. The first component is real, the second
imaginary, each an integer in `[0,p)`. Singular keys and `A=0` are refused.
The accepted key bytes are its identity for the challenge hash.

The signature is `A_sig || Px || Py || Qx || Qy || salt`, five fp2
encodings and exactly the row's salt length. Field components are not reduced
on decoding. Every salt byte string of that length is admissible: salt
binding comes from the hash, not a unique numerical salt representation.
Truncation, extension and out-of-range field elements are refused. The
archived two-byte-index encoding and the paper's C wire encoding are not
accepted by default verification.

## Challenge and signing

`s = cSHAKE256(S="shekyl-prism-challenge-v2";
A_bytes || salt || message)`, truncated to `e−3` bits with bit zero set.
The leading key and salt lengths are fixed for the row. `q=2^(2e−5)−s²`.
Public verification tests that single degree and never searches salts.

The degree test remains deterministic Baillie–PSW: small-prime sieve
(the primes below 100), strong base-2 Miller–Rabin and strong
Lucas–Selfridge. It is a probable-prime predicate, not a certificate;
adversarial adequacy remains unresolved.

Selfridge's discriminant is the first of `5, −7, 9, −11, …` whose Jacobi
symbol modulo the degree is `−1`, searched over 256 candidates. A shared
factor, or no such candidate among the 256, is a refusal. There is no
separate test for a perfect square: the Jacobi symbol modulo a square is
never `−1`, so the search ends empty on every square and the Lucas step
refuses it. A degree `2^(2e−5) − s²` with `s` odd is `7 (mod 8)` and is
never a square, so the case does not arise in verification. No loop of
the test ends on a computed value: each is counted, so no input and no
single wrong operation makes the test fail to return.

The [salt protocol contract](SALT_PROTOCOL.md) specifies the 32-byte caller
randomness input and the secret-keyed cSHAKE salt schedule. Research signing
uses zeros by default; a caller may provide fresh randomness without
changing verification. The internal counter is never encoded. Signing
visits at most 16 prime candidates and 65536 counters, skips candidate
backend errors, malformed responses and failed final verification, and
returns only exact bytes accepted by Rust. Initial setup/keygen errors
remain typed errors. Exhaustion returns no signature. The response stream
remains deterministic for each key and scalar. The PRF, rejection/failure
distribution, and stronger isogeny assumptions are still review obligations.

## Normalization

The signature must satisfy `0 < y(P_sig) < y(−P_sig)` in the fp2 order:
compare the real components as integers first, then the imaginary ones.
The signer negates both points together when necessary. This follows
Algorithm 2's convention, correcting the opposite inequality printed in
Algorithm 3 line 1.

`A_sig²=4` is refused as Singular. The response coefficient must be the
least coefficient in its Montgomery isomorphism class, under the same fp2
order. The candidates are `±A` and `±(3α+A)/λ` for the two nonzero rational
2-torsion roots `α`, with `λ²=α²−1`. Full rational high-order torsion
ensures the required rational transports exist for an honest signature.
A response curve whose `A²−4` is not a square, so that those roots are
not rational, is refused as Model; a root whose `λ` is not rational
gives no candidate and is passed over.
The exporter transports y using `λ^(3/2)` before joint normalization.

The response classes `j=0` (`A²=3`) and `j=1728` (`A=0` or `2A²=9`) are
excluded as Model. Choosing a coefficient alone cannot normalize their
extra automorphisms. This is an explicit restriction of the research
encoding; a signer skips such a response. No claim of support for those
classes is made.

## Canonical public-key basis

The bounded C-reference basis-index search is unchanged: search from 1
through the set's search bound, including the C's seven-bit hint fallback
in one search.
The verifier carries no basis hint byte. The square test on `1+b²` is in
`F_p`, not over the integers.

Let `P` be the index point cleared by the odd cofactor `f`, and `R` the
point with uncleared abscissa `−x(index point)−A`, also cleared by `f`.
The signature pair is `(P,Q)` with `Q=difference(P,R)` **after** clearing.
`P−Q` has abscissa `R`. In the existing Rust `EvenBasis` names these are
`even.p`, `even.difference`, and `even.q`, respectively. Lift this triple
together to fix the relative signs; a common sign does not matter.
The transitional single-chain probes are retained only in tests.

`difference` is the C reference's `difference_point`. With `Bxx`, `Bxz`
and `Bzz` the biquadratic forms of the curve at the two points, each
multiplied by the squares of the conjugates of the two `Z` coordinates,
it is the point `(Bxz + √(Bxz² − Bxx·Bzz) : Bzz)`: one of `x(P−R)` and
`x(P+R)`. Two things in that are part of the protocol, because the other
choice gives the other point as `Q`. With the other square root, the
tests that verify the committed honest signatures fail:

- **The square root.** It is the root whose real part is even as an
  integer in `[0, p−1]`, or whose imaginary part is even when the real
  part is zero. That is the rule of the reference's `fp2_sqrt`, and
  `prism-field` promises it for the extension field of every set and
  tests it there.
- **The coordinates.** The rule reads a parity, and a parity is not
  preserved when both coordinates of a point are multiplied by a common
  factor. For the same two points written in another projective scaling,
  `difference` returns the other candidate about half the time (ledger
  O21). `P` and `R` are therefore taken exactly as the Montgomery ladder
  by `f` returns them, from `(x : 1)`, without normalization, which is
  what the C reference's basis functions do. A signer and a verifier are
  sure to agree on `Q` only when they compute it from the same
  coordinates.

## Acceptance relation

Both public-key basis points must have exact order `2^e` and span the
public even torsion; the search alone is not enough. Both response points
must be on `E_sig`, have exact order `2^e`, and span
the full even torsion. Before constructing either chain require

`e_2^e([s]P_sig,[s]Q_sig) = e_2^e(P,Q)^q`.

Since `q ≡ −s² mod 2^e`, use the low `e` bits of `−s²` for this comparison.
Pairing constrains the determinant; it does not establish the isogeny or
bind the message by itself.

Use the symplectic product basis
`(P, [s]P_sig, Q, [q^−1 s]Q_sig)`. Build the two closed-form symplectic
matrices recorded in the archived dual-structure derivation, with
`t=s^−1 mod 2^e`. Their kernel halves give full-point lifts, not x-only
kernels. The long chain consumes order `2^e` points; the short chain
consumes their appropriate order `2^(e−1)` multiples.

Canonicalize each factor's order-four basis while preserving its pairing.
A bounded enumeration over four coefficients replaces the Sage reference's
order-four discrete logarithms. Build the product theta null and apply
`base_change_theta(M0*Mi mod 4, e4)` to each chain; it has no result when
none of its four shifts gives a nonzero sum, and the verifier then refuses
with `Kernel`. No signature reaches that case (the reason is with the
function). The adapted gluing,
8-torsion steps and dual evaluation are ported from the pinned Theta_dim4
reference. The balanced kernel stack uses O(log e) saved generator pairs.

Require `C1.zero() == H(C2.zero())` projectively. Then evaluate
`F = dual(chain2) ∘ chain1` on `(X,0)` for `X=P,Q,P−Q` and require

`x(F(X,0)[0]) = x([s]X)` and
`x(F(X,0)[1]) = x([s]Y)`,

where `Y=P_sig,Q_sig,P_sig−Q_sig`, respectively. This is the executable
Sage model's complete two-component image criterion. The difference point
constrains relative signs. The mathematical soundness and uniqueness
argument for this signed representation remains subject to external review.

Every projective comparison rejects an all-zero coordinate vector. The
inverse gluing map also requires a nonzero product point satisfying its
rank-one relation; it does not manufacture infinities from invalid output.

## Refusals and cost

Wrong lengths precede decoding. Key checks and canonical decoding precede
Sign, response Singular/Model, bounded Basis and Composite. Then run
on-curve/order/span checks, Pairing, chain construction, Codomain and Image.
`Kernel` can occur before or after pairing, and `Singular` can refer to
either curve; enum ordering is not a global precedence promise.
`Pairing` is the refusal of the pairing equation, and also of a pairing
that has no value and of a key basis that does not lift to full points;
no input is known for the last.
`Ok(())` is returned only after every image comparison passes.

The crate documentation of `prism-verify` has the refusals one to a
row, in the order the byte-level entry makes them. Which of those rows
a test reaches is ledger O23.

Verification has no RNG or secret inputs. Public arithmetic is variable
time. The signer remains a separate crate and process backend.

## The two entries

The verifier has two entries, and they accept the same inputs.

**The byte-level entry**, `prism_verify::verify::<L>(public_key, message,
signature)`, takes three byte strings and runs every check on every call,
in the order of the section above. It is the definition: its signature,
its refusal order and its operation counts are held fixed, and the
committed count records are taken through it.

**The typed entry** is the interface of the `shekyl-prism` crate. It
splits the same checks by the object they depend on:

| Call | Checks | Refusals, in order |
| --- | --- | --- |
| `PublicKey::<S>::from_bytes` | every check on the key alone: length, encoding, the two excluded curves, the search for the basis index, and that the points the index names are a basis of `E[2^e]` | `Length`, `Canonical`, `Origin`, `Singular`, `Basis` |
| `Signature::<S>::from_bytes` | every check on the signature alone | `Length`, `Canonical`, `Sign`, `Singular` or `Model` |
| `PublicKey::verify` | the checks that need key, message and signature | `Composite`, then `Kernel`, `Pairing`, `Codomain`, `Image` |

A value of either type passed its checks, and there is no other way to
make one. The key keeps the basis its construction found and tested, so
the relation neither searches for it, computes it nor tests it again.
`Basis` is the refusal of a key that gives no usable basis, by either
cause: the search finds no index within the bound, or the points the
index names are not a basis of `E[2^e]`, as on a curve whose
`2^e`-torsion is not rational. The three calls are
`PreparedKey::prepare`, `checked_response` and `verify_prepared` of
`prism_verify::prepared`. Those are low-level functions, and they hold
the same thing in types of their own: a `PreparedKey` and a
`CheckedResponse` can each be made only by the function that checks it,
and `verify_prepared` takes nothing else.

The two entries share their definitions. The signature's checks are one
function, which the byte-level entry calls after its length checks and
the key's decoding and the typed entry calls alone. Primality followed by
the relation is one function, and the relation has one body. The test of
the key's basis is one function too, the only way to the type both
entries hand to the pairing. What differs is the order in which the
shared pieces run, and whether the key's basis is built on the call or
carried.

The byte-level entry looks at both lengths, decodes the key, then decodes
and checks the signature, and only then searches for the key's basis. It
tests that basis later still: after the challenge and its primality, and
after the signature's own points. The typed entry finishes the key, the
search and the test included, before any signature exists. So, with the typed entry taken as its three calls in the order
key, signature, verification, and its refusal as the first of them:

1. the typed entry accepts exactly when the byte-level entry accepts;
2. if `PublicKey::from_bytes` succeeds, the typed entry's refusal is the
   byte-level entry's;
3. if `PublicKey::from_bytes` fails, the byte-level entry also refuses,
   and may name another error. The typed entry names the key's fault.
   The byte-level entry names `Length` when the signature's length is
   wrong, whatever the key is. For a key whose only fault is `Basis` it
   names any fault of the signature (`Canonical`, `Sign`, `Singular`,
   `Model`) first; and when the fault is that the named points are no
   basis, it names `Composite` if the challenge degree is composite, and
   otherwise `Kernel`, its name for points that are no basis on either
   curve.

Case 3 covers every key the typed entry refuses. For the four faults of
a key before `Basis` it needs a faulty signature as well: with a good
signature both entries name the key's fault. A key refused as `Basis`
is named differently by the byte-level entry whatever the signature is,
since that entry has no `Basis` refusal later than the search.

The cost order differs with it. The basis search is the one check whose
length the sender of a key chooses, up to the search bound, and the test
of the basis is the dearest of the key's checks: about `6e` doublings
on the key's curve, some 6,000 multiplications and 3,000 squarings in
the extension field at `p248`, 3.9% of a verification's products
(`docs/BENCHMARKING.md`). The byte-level entry runs the signature's
cheap checks before the search and the challenge's primality before the
test; the typed entry runs both when the key is built, and never again.
That suits a key validated once and kept, and not a key built from every
incoming message.

## Tagged containers and the accept list

A caller that names the prime in its source uses the entries above. A
node that reads keys and signatures from the wire or from stored state
needs the bytes to say which prime they are under, and has to say which
primes it accepts. The front crate adds an identifier, a container that
carries it, and a list.

**The identifier**, `shekyl_prism::SetId`, is the pair `(e, f)` of the
prime `p = f·2^e − 1`: `e` and then `f`, each a little-endian 16-bit
integer, four bytes.

| Set | `e` | `f` | Identifier bytes | Tagged key bytes | Tagged signature bytes |
| --- | ---: | ---: | --- | ---: | ---: |
| `p248` | 248 | 5 | `f8 00 05 00` | 68 | 356 |
| `p324` | 324 | 3 | `44 01 03 00` | 86 | 462 |
| `p376` | 376 | 65 | `78 01 41 00` | 100 | 532 |
| `p500` | 500 | 27 | `f4 01 1b 00` | 132 | 708 |
| `p664` | 664 | 17 | `98 02 11 00` | 172 | 940 |

A `SetId` value names one of these five. Four bytes that name none of
them do not become one; the refusal carries the `e` and `f` read. Each
set has one identifier and each identifier one encoding.

**A tagged public key** is the identifier and then the bare key of the
section "Parameters and encoding". **A tagged signature** is the
identifier and then the bare signature. Each is exactly that length:
nothing may follow. The bare encodings are unchanged, and a tagged value
has one encoding because the bare one has.

**The accept list**, `shekyl_prism::AcceptList`, is the caller's policy:
the sets it accepts, written out. It has no default and there is no list
of every set; an empty list is refused where it is made.

**The tagged entry**, `shekyl_prism::verify_tagged(accept, public_key,
message, signature)`, takes a tagged key and a tagged signature and
returns the set it accepted. It refuses at the tag layer first, in this
order, each before the next:

1. `TooShort`: an input is shorter than an identifier. The key is looked
   at before the signature;
2. `UnknownSet` for the key: its identifier names no built set;
3. `UnknownSet` for the signature;
4. `Mismatch`: the two identifiers differ. It names both;
5. `NotAccepted`: the set is not in the caller's list.

Then the two bare parts go to the byte-level entry of that set,
unchanged, and its `VerifyError` comes back wrapped and unaltered. The
byte-level entry is the one whose check order suits a key and a signature
that arrive together: it checks the signature's encoding before it
searches for the key's basis.

The five refusals of the tag layer read at most the first four bytes of
each input and cost no field operation, so no sender can make the entry
work before the list has been consulted.

**The typed readers.** `PublicKey::<S>::to_tagged` and
`Signature::<S>::to_tagged` write the tagged form. `from_tagged` on
either type reads it and refuses, before any check of the bare bytes,
an input shorter than an identifier (`TooShort`), an identifier that
names no set (`UnknownSet`), and an identifier of a set other than `S`
(`WrongSet`); then the bare bytes are checked as `from_bytes` checks
them.

**What the identifier is not bound by.** The challenge transcript does
not contain the identifier. What stops a key and a signature of one set
being presented under another set's identifier is their length: the five
sets' bare key lengths are pairwise different, and so are their bare
signature lengths, so bytes of one set under another's identifier are
refused by the byte-level entry as `Length`. That holds for these five
primes. It would not hold for two primes whose encodings have equal
lengths. A test fails if a set is added whose key length or signature
length repeats another's, and adding such a set reopens the question of
putting the identifier in the transcript (ledger O16).

**What the identifier does not say.** It says which prime the bytes are
under. It does not say the key's owner chose that prime. A system that
accepts more than one set must bind the set to the key where the key is
created or published, and must not let the identifier on a signature
choose the set. A list of more than one set is as strong as its weakest
member. A system that needs a signature bound to its set beyond what the
length gives puts the identifier in the message it signs. The refusals
`UnknownSet` and `NotAccepted` are different errors, and neither is a
secret: both are facts about public bytes and a public policy.

## Executable evidence

- Tagged equals bare: `shekyl-prism/tests/support/entries.rs` also runs
  `verify_tagged` on every input it is given, with the set's identifier
  in front of the key and of the signature and a list of that one set,
  and fails unless it returns the set exactly when the byte-level entry
  accepts and otherwise the byte-level entry's error, wrapped. That is
  every input of the next item. The tag layer itself is tested in
  `shekyl-prism/tests/tagged.rs`: all 25 ordered pairs of sets, a tag
  over another set's bytes for each of the 20 unequal pairs, the six
  honest primary vectors retagged under each of the four other
  identifiers, the list with and without each set, 37 identifiers that
  name nothing, every length below four, the refusal order with two
  faults at once, and the pairwise different lengths.
  `shekyl-prism/tests/tag_layer_counts.rs` shows with the operation
  counter that every tag-layer refusal costs no field operation; it
  requires the counter, so the `op-counts` job runs it and a plain
  `cargo test` does not.
- The two entries are held to the rule above on every input the tests
  of the front crate verify: `shekyl-prism/tests/support/entries.rs` runs
  both and fails unless the rule holds, and those tests call the verifier
  through it. That is the 113 honest signatures and the 2,742 negatives
  derived from the six primary vectors (`shekyl-prism/tests/acceptance.rs`,
  which pins both counts), the committed identity refusals at all five
  sets, and every refusal built without a signer at all five sets. A
  faulty key with a faulty signature, in every pairing of the faults that
  need no signer, is built at each set with the refusal of each entry
  pinned (`shekyl-prism/tests/entries_agree.rs`). The `Basis` key there
  is one whose named points are no basis, and 199 such keys at each of
  the five sets are run through both entries with a signature good in
  itself. No key refused as `Basis` for want of an index is known, so
  that cause is argued from the order of the code and not run.
- The inputs are built by `prism-testkit`, which depends on the verifier
  and not on the front crate. The tests are in the front crate because a
  test lives in the lowest crate that contains everything it exercises,
  and no dependency edge points up, dev edges included
  (`scripts/ci/check_no_upward_edge.py`). `cargo test -p prism-verify`
  stands without the front crate: its unit tests, the Sage comparisons of
  the sets without a corpus, and `prism-verify/tests/committed.rs`, where
  `verify` accepts the 113 honest signatures and `verify_set` returns the
  pinned error on the ten committed identity refusals.

- All 113 committed honest signatures are executed through `verify`.
- Both chains match every Sage theta-null point for the six primary vectors.
- Negative tests cover signs, swaps, determinant-one shears, odd scalar
  changes, every nonzero order-four translate in either or both points,
  alternate prime challenges, other messages, all five alternative models,
  reduced orders, off-curve points, singular and exceptional models.
- Salt negatives. Every one-bit change of the salt is refused: `Composite`,
  or `Image` when the new degree is prime. Over all 113 signatures the
  split is 62 prime to 5,570 composite at I, 137 to 18,679 at III and 118
  to 21,386 at V. A salt found by search whose degree is prime gives
  `Image`. The salt of another valid signature under the same key gives
  `Composite` or `Image` (native signer test). An archived unsalted
  signature gives `Length` (`prism-compare`).
- Test builds use optimization level 1 with debug assertions and overflow
  checks enabled, so the full crypto corpus remains practical in CI.
- The native signer test generates fresh responses at all levels, checks
  byte-for-byte determinism, binary messages and message binding, with
  zero and nonzero caller randomness.
- The default build has no reduced-width entry point: CI compiles a
  consumer that must fail on `prism_verify::research` and build only with
  `research-reduced-width`. At the production width the comparison driver
  calls the production signing session, so the bytes it releases are the
  production signer's bytes.
- `scripts/sage/differential.sage` compares a fresh Sage execution against
  Rust, including the other-key cases. It requires Sage and pinned source.
- Fuzz corpora are seeded with all honest signatures, so deep paths are
  reachable. Fuzzing is not a mathematical soundness proof.

The benchmark uses an accepted honest signature at each set. Development
machine numbers are not a Raspberry Pi measurement. See
[benchmark instructions](BENCHMARKING.md).
