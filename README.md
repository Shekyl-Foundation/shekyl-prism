# shekyl-prism

Research implementation of the prime-degree §5 PRISM signature relation,
with a Rust verifier and an experimental deterministic native signer.
It is not a consensus crate or an adopted wallet authorization scheme.
The exact protocol still needs cryptographic review; see
[open security items](docs/SECURITY_REVIEW.md).

| Crate | Role |
| --- | --- |
| `prism-field` | Three field rows, scalar `fp2` 0.6.0 backend |
| `prism-isogeny` | Montgomery and theta arithmetic, adapted chains and duals |
| `prism-verify` | Canonical decoding, pairing, both chains and complete images |
| `prism-signer` | Unix research API for an isolated deterministic C backend |
| `prism-bench` | Successful complete verification at I, III and V |

The [verifier specification](docs/VERIFIER_SPEC.md) defines the current
research encoding, including the `e′=e−2` parameter choice. Signatures are
322 / 482 / 642 bytes. I/III/V name the source field rows, not certified
security levels for this modified unsalted construction.

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
the accept-list script checks only provenance. The nightly fuzz corpora
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
bounded deterministic challenge search, normalizes the response, and
verifies every result before returning it. Native failure returns an error.
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
`PRISM_DIFFERENTIAL_LEVEL=NIST-III` to restrict it to a parameter row.
The original model report is in
[REPORT.md](prism-isogeny/testdata/s5_model/REPORT.md).

The benchmark now times accepted signatures. Development-machine results
and the Raspberry Pi 4 command are in [BENCHMARKING.md](docs/BENCHMARKING.md).
A Pi measurement and independent cryptographic review are still adoption
gates. Paper sources and the pinned C-reference provenance are described
in [C_REFERENCE.md](docs/C_REFERENCE.md).
