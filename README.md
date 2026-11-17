# shekyl-prism

PRISM signatures in pure Rust: an implementation of the prime-degree §5
PRISM signature relation, with a salt. A signature is an isogeny of prime
degree from the signer's public curve, and the degree is bound to the
message by a hash. The repository holds a Rust verifier at five parameter
sets, the committed test vectors it is held to, and the tools that
measure it. It is written to be read, measured and extended by anyone
working on the scheme, and to become the signature implementation Shekyl
uses.

## Status

**Research code. Unreviewed. Not for protecting anything of value.**

- The exact protocol still needs cryptographic review; see the
  [open security items](docs/SECURITY_REVIEW.md). It is not a consensus
  crate or an adopted wallet authorization scheme.
- The repository is in **alpha on `dev`** until it has a solid verifier
  and a signer in Rust. A **beta then goes to `main`**. Until then the
  interface can change from one change to the next.
- Verification is in Rust. Signing is not yet: the present signer is an
  experimental wrapper around the C reference implementation, with a
  32-byte caller randomness input (zeros in reproducible research runs).
  It exists to produce test vectors.

The [security argument draft](docs/SECURITY_ARGUMENT.md) maps the salted
implementation to its algebraic premises, exact parameter bounds,
second-preimage cost, and conditional SUF-CMA decomposition. The
[salt comparison](docs/SALT_COMPARISON.md) records what the unsalted
protocol cost an attacker, what the salted one costs, and what the salt
does not change, with both attacks run against the real signer and
verifier at reduced challenge widths.

## Quick start

```rust
use shekyl_prism::{PublicKey, Signature, P500};

let key = PublicKey::<P500>::from_bytes(&key_bytes)?;
let signature = Signature::<P500>::from_bytes(&signature_bytes)?;
key.verify(message, &signature)?;
```

A key and a signature are checked when they are made, so a value of
either type is valid by construction, and both carry their parameter set
in their type.

Where the bytes have to say which prime they are under, a key or a
signature carries a four-byte set identifier in front, and the caller
says which sets it accepts:

```rust
use shekyl_prism::{verify_tagged, AcceptList, SetId, P500};

let accept = AcceptList::new(&[SetId::of::<P500>()])?;
let set = verify_tagged(&accept, &tagged_key, message, &tagged_signature)?;
```

There is no default list and no list of every set. A list of more than
one set is as strong as its weakest member, and the identifier does not
say the key's owner chose that prime; `AcceptList`'s documentation says
what follows from both.

The crate documentation (`cargo doc -p shekyl-prism --open`) has
examples that run against the committed vectors, and
`shekyl-prism/examples/` has three programs: `verify`, `verify_tagged`
and `read_vector`.

## The crates

Three tiers. Depend on the first; read and extend the second; the third
is what tests and measures them.

**The interface**

| Crate | Role |
| --- | --- |
| `shekyl-prism` | Typed keys and signatures, valid by construction, and verification; the set identifier, tagged keys and signatures, and verification under a caller's accept list. Every item is documented. The three crates below are re-exported under its `hazmat` module |

**The low-level layer**: where the algorithm is. Public and documented, but
its functions assume invariants that only the interface checks in full,
and nothing in it is promised stable.

| Crate | Role |
| --- | --- |
| `prism-field` | Five parameter sets (`p248`, `p324`, `p376`, `p500`, `p664`), scalar `fp2` 0.6.0 backend |
| `prism-isogeny` | Montgomery and theta arithmetic, adapted chains and duals |
| `prism-verify` | Canonical decoding, pairing, both chains and complete images; the byte-level entry `verify` and the entry split by object that the interface is built on |

**Supporting crates and tools**

| Crate | Role |
| --- | --- |
| `prism-vectors` | A public crate: every committed test vector and a documented reader for each family. Depends on `prism-field` alone, so the data can be used without a verifier; the library crates take it as a dev-dependency only |
| `prism-testkit` | Test inputs as a documented library: the 2,724 negatives derived from the committed vectors and the responses built from public data alone. A verifier or a signer under test takes it as a dev-dependency. Depends on the three layer crates and `prism-vectors`, never on `shekyl-prism`; the three reporting tools (`op_counts`, `verify_once`, `identity_refusals`) are its examples |
| `prism-signer` | Unix research API for an isolated deterministic C backend |
| `prism-bench` | Timing: an accepted verification at the sets with a signer, and a refusal after the pairing check at all five |
| `prism-compare` | Archived derivation and explicit research experiments; no production dependency |

## The protocol and its parameters

The [verifier specification](docs/VERIFIER_SPEC.md) defines the current
research encoding, including the `e′=e−2` parameter choice. Signatures are
352 / 528 / 704 bytes at `p248` / `p376` / `p500`, including salts of 32 / 48 / 64 bytes.
A parameter set is a prime and is named by it; the category a prime is
assessed at is documentation, in [PARAMETER_SETS.md](docs/PARAMETER_SETS.md),
and is not a certified security level for this modified salted construction.
Two of the three primes with vectors are from before the `p^(1/3+o(1))` isogeny
algorithm of ePrint 2026/1486, which applies to PRISM key recovery and after
which SQIsign moved to larger primes. Ruled 2026-11-14: the sets are the five
primes of that table and Shekyl uses `p500`. `p324` and `p664` are built on the
verifier side only, with no signer and so no accepted signature ([SECURITY_ARGUMENT.md](docs/SECURITY_ARGUMENT.md) §9). The approved wire
and signer contract is [SALT_PROTOCOL.md](docs/SALT_PROTOCOL.md); the
unsalted implementation is preserved at `archive/unsalted-index-2026-11-12`.

The verifier accepts all 113 committed honest signatures. Tests compare
both chains against every intermediate null point from six primary Sage
traces and reject sign, basis, torsion, model and message mutations. The
least signature-curve model is enforced. Response curves with exceptional
automorphisms are excluded. An all-zero projective vector is never equal
to a valid point.

## Rust checks

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo run --release --locked -p prism-bench
```

Workspace tests execute the actual verifier on the entire honest corpus;
`scripts/ci/check_accept_verdicts.py` checks only provenance. The nightly fuzz corpora
include honest signatures so that deep verification paths are reachable.

## Native signing

The backend is currently built for Linux x86_64. Initialize the pinned
submodules before these commands:

```
python3 scripts/check_c_reference.py
cmake -S external/PRISM_v2 -B target/native -DCMAKE_BUILD_TYPE=Release -DPRISM_BUILD_TYPE=ref -DGMP_LIBRARY=MINI
cmake --build target/native --parallel 4 --target prism_lvl1 prism_lvl3 prism_lvl5
PRISM_REF_BUILD="$PWD/target/native" sh scripts/export_s5_vectors.sh backend
PRISM_SIGNER_BACKEND_DIR="$PWD/target/native" cargo test -p prism-signer --release --locked --test native -- --ignored
cargo run --release --locked -p prism-signer --example roundtrip -- target/native
```

`prism_signer::Signer` owns a zeroized 48-byte master seed and derives
separate cSHAKE256 keygen and per-degree signing seeds. The backend consumes
those seeds over stdin and uses SHAKE256, not the fixture generator's
historical xorshift stream. It applies the positive-cofactor Cornacchia
patch to a copy of the pinned quaternion archive. The original source and
fixture RNG are retained for reproducibility. The libraries are not linked
into the Rust verifier.

The API regenerates the native secret key in private scratch, uses a
bounded deterministic salt search, normalizes the response, and verifies
every result before returning it. A native failure at one candidate moves
to the next salt; setup failure or exhaustion of the bounds is an error.
Normal exits remove the scratch directory. No constant-time or complete
secret-erasure claim is made; native side-channel review remains required.
No wallet keystore, output-key derivation or rerandomization is included.

## Independent model and measurements

With Sage installed and pinned submodules initialized:

```
cargo build --release --locked -p prism-verify --bin s5_challenge --bin verify_bytes
sage scripts/sage/differential.sage
```

This fresh comparison does not rewrite committed known answers. Set
`PRISM_DIFFERENTIAL_LEVEL=NIST-III` to restrict it to one parameter set.
The value is the `level` field the vectors carry: `NIST-III`, which is
`p376`.
The original model report is in
[REPORT.md](prism-vectors/data/isogeny/s5_model/REPORT.md).

The comparison experiments are `prism-compare` binaries behind the
`experiments` feature. `sh scripts/run_comparison.sh <record> <driver>
[arguments]` runs one from a committed tree and writes its raw output and
provenance under `docs/comparison/`;
`python3 scripts/analyze_comparison.py` recomputes every table of the
comparison document from those records.

The benchmark times accepted signatures at the three sets that have
them, and at all five sets a committed response that runs both chains
and is refused. Development-machine results
and the Raspberry Pi 4 command are in [BENCHMARKING.md](docs/BENCHMARKING.md).
The Pi measurement is recorded: `p376` takes about 449 ms in the
captured conditions, for the unsalted verifier before its operation count
was cut. That count, per stage and pinned in CI, and the times before and
after on a development machine are in the same document. Independent cryptographic review and a deployment
verification budget remain adoption gates. Paper sources and the pinned C-reference provenance are described
in [C_REFERENCE.md](docs/C_REFERENCE.md).
