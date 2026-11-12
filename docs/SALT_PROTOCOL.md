# Salted research protocol

Status: LIVING CONTRACT, approved brief and caller-randomness amendment
2026-11-12; implementation and experiments in progress. Base: PR #14 merge
`e6f90bccd4630f81aa15b6120fc874d5312851a5`. This specifies a research
protocol, not wallet or consensus adoption. The unsalted executable remains
at `archive/unsalted-index-2026-11-12` (`d018757`).

## Wire and challenge

Each signature is `A_sig || Px || Py || Qx || Qy || salt`, with five
canonical fp2 encodings and exactly 32/48/64 salt bytes at I/III/V. Total
lengths are 352/528/704 bytes. All salt byte strings of the required length
are allowed. There is no index and no counter in the wire or public hash.
Hash binding, rather than a distinguished salt encoding, binds its value.

The public challenge is cSHAKE256 customized by
`shekyl-prism-challenge-v2`, absorbing `pk_bytes || salt || message`.
The leading two fields have fixed row-dependent lengths. Project to the
odd positive integers below `2^(E−3)`, exactly as before; the free-bit
counts stay 244/372/496. Put `r=E−2`, `q=2^(2r−1)−s²`. BPSW, field
parameters, canonical model/sign checks, torsion checks, pairing, dual
chains and all image checks are unchanged. Public verification derives
one challenge from the carried salt and never searches.

## Signing and caller randomness

The signer takes a caller-supplied 32-byte randomness input R. The research
API's default and committed vectors use all zeros; a wallet's default is
outside this work. The signer itself draws no fresh randomness for salts.
Keep the existing master-seed/level/label secret-stream derivation and use

```text
salt_key = stream_seed("salt", "")
salt_i = cSHAKE256(S="shekyl-prism-salt-v1";
                  salt_key || pk_bytes || R || LE64(i) || message)[..SALT_LEN]
```

The randomness is fixed for one signing call. For counters 0 through 65535,
derive a salt and challenge. Visit at most 16 BPSW-passing candidates.
Attempt the native response using the unchanged scalar-keyed signing
stream, normalize and run the exact Rust verifier before returning bytes.
A native candidate error, malformed response or failed final verification
advances the counter. Initial keygen/setup errors remain typed errors.
Exhaustion has no fallback response. Record aggregate candidate/backend
failure counts for the research comparison without outputting the counter
or secret material in a signature.

The zero-input mode is reproducible for a fixed backend. Fresh caller
randomness varies runs. Under the relevant PRF assumptions the hedge can
retain salt unpredictability when one entropy source fails; it does not
prove fault or side-channel resistance. Auditing a hedged salt requires
the caller randomness as well as the seed. No claim that arbitrary
caller-selected randomness prevents a covert channel is made.

## Comparison and feature boundary

`prism-compare` owns archived unsalted challenge/search logic and experiment
drivers. Neither production crate depends on it. Only the off-by-default
`research-reduced-width` verifier feature exposes an entry point accepting
a smaller challenge width or an archived response. Default `verify` never
uses that entry point. Production signing does not enable the feature.
The field crate contains no archived index type or layout constant.

The comparison changes only the projection width, retaining real field
sizes, degree arithmetic, native signing and all algebraic verification.
For projection width w the forced odd bit leaves w−1 free bits. Record both
to avoid an off-by-one entropy claim. E3 uses 20 trials at each projection
width 16/20/24/28/32. The salted slope near one refers to a fixed one-signed-
target second-preimage budget, not unrestricted multi-target attacks.

Record raw, commit-pinned measurements E1–E6 from the brief, including
failed trials, actual query counts, seeds, commands, fitted slopes,
backend retry counts and the E6 search budget if no failure is found.
The user authorized documented replacement seeds for the 36 older numbered
vectors whose SHA-256 seed recipe was absent from Git. The other 77 retain
the recovered original fixture streams. The seed manifest identifies every
replacement and hashes the original vector preserved in the unsalted copy.
Never turn a Derived prediction into a Ran claim without executing it.
If an experiment refutes a Derived prediction, stop and report it.

The comparison distinguishes BPSW eligibility from usable native signing
acceptance and hash-query exponents from AES-equivalent security claims.
Map the salted cost formula to Lemma 1, Theorem 2 equation (12), and its
cost-normalized consequence equation (18), adding the deterministic/hedged
PRF and failure-distribution obligations. The theorem's EUF-CMA statement
does not discharge the stronger isogeny assumption or A1–A3 needed here.

## Delivery record

One PR on `protocol/salt`, pushed in logical units. Production conversion,
same-seed vector regeneration, fresh Sage comparison, default-feature
isolation checks and CI precede the manual E1/E3 runs. Final documentation
updates cover SECURITY_ARGUMENT, VERIFIER_SPEC, SECURITY_REVIEW, README,
NOTICE and SALT_COMPARISON in the order specified by the approved brief.
The comparison must explicitly retain signer-selected message collisions,
predictable salts, and the unchanged primality/map/encoding obligations.
Only the user merges the PR.
