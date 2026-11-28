# Can the second signer change after the first signer goes offline?

**Yes for ordinary salted PRISM approvals over an enrolled policy.** Alice signed one payment once. The verifier accepted both Alice + Bob and Alice + Carol using exactly the same Alice approval. As a control, Alice also signed a version that names Bob. That signature verified with Bob named and failed with Carol named. All three honest-fixture sets gave both results. This distinction matters to the person approving a payment: “any two members of this group” and “only Alice and Bob” are different instructions.

## Evidence

| Set | One approval | Two-member witness | Result |
| --- | ---: | ---: | --- |
| P248 | 425 B | 850 B | All three pairs accepted, fixed-pair control passed |
| P324 | 549 B | 1,098 B | Format and invalid-signature refusal only |
| P376 | 633 B | 1,266 B | All three pairs accepted, fixed-pair control passed |
| P500 | 841 B | 1,682 B | All three pairs accepted, fixed-pair control passed, private onion run |
| P664 | 1,113 B | 2,226 B | Format and invalid-signature refusal only |

The experiment starts at shekyl-prism research revision 1cc5d26. The test generator minted new signatures for this payment with the repository's existing reference signer. The verifier, collector and onion driver are Rust. P324 and P664 have verifier support and no reference signer at this revision. The original fixture message from the verifier corpus was not used.

Each honest set also refused changed network, input, recipient, amount, fee, expiry, and a policy change affecting only the absent member. Other checks cover duplicate enrollment, repeated delivery, a false signer index, a corrupted signature, truncated/extra approvals, and duplicate or out-of-order positions. A second copy counts once. A verification failure leaves prior approvals available.

## What the experiment authorizes

The input has one policy of three **independently enrolled** tagged keys in a fixed order. This public fixture uses one input, one recipient, amount 970, fee 30 and expiry height 12345. The network, input and recipient are distinct synthetic 32-byte identifiers. In a wallet the signer must reconstruct this information and resolve the actual input policy independently. The experiment supplies it as a trusted fixture before processing either witness.

Let H(label, body) be SHA3-256 over the ASCII label concatenated with body. Every label below starts with `prism-lab/quorum/` and ends in `/v1`. The bodies concatenate fixed-width fields in the following order.

| Label | Body |
| --- | --- |
| key | Tagged PRISM public key |
| policy | Four-byte PRISM SetId, threshold byte 2, member-count byte 3, three 32-byte key hashes in enrollment order |
| payment | Network, input, policy hash, recipient, amount, fee, expiry height. The last three are little-endian u64 |
| fixed-pair | Payment digest and two signer positions, one byte each |

A member signs the payment digest. Its public approval contains the slot byte, tagged public key and tagged signature. A witness is exactly two approvals in ascending slot order. The comparator signs the fixed-pair digest. Enrollment and witness are delivered separately. Enrollment checks the set, encoded length and distinct key hashes. Signature verification uses the repo's tagged Rust verifier, with the policy's one permitted set. Enrollment does not prove key ownership or device independence.

A separate calculation using Python hashlib on the P500 enrollment gives policy commitment `83e5ff15e6fa664b350a4c3643f1badc314105371e61688fd197b0288bc9ecda` and payment digest `e6d8c89beff2e04dfbeda3cd2b49bd3a69bc48496c793066eabbfc2ba64fd129`.

## Private onion run

A real Tor 0.4.9.15 client fetched and posted the public P500 approval frames through a v3 onion service on Chutney's private `hs-v3-min` network. Three authorities, three non-exit relays, a client and a service ran on loopback. A separate in-memory mailbox process retained a complete upload independently of the original signing process.

1. The client uploaded Alice's pre-signed 841-byte frame. It cut Bob's connection after 420 of 841 body bytes. Reading Bob's slot returned empty.
2. The assembler retrieved Alice's frame over onion, saved it to a file and imported it again. It still had one approval. A fresh collector reconstructed that state from the saved public frame, then completed with Carol's file.
3. The client uploaded Carol's identical frame twice and fetched it over onion. The online collector and the file collector produced **byte-identical 1,682-byte witnesses**, each independently accepted by the Rust verifier. One run took 1.39 seconds in total. This is an observed loopback run with precomputed approvals, not a public-network or signing latency estimate.

The mailbox carries **public test fixtures in clear to the mailbox operator**. It has three fixed slots and serves no production data. The slot owner is checked by the receiving verifier. Any writer reaching this laboratory service could occupy a slot and deny delivery. A usable asynchronous service still needs authenticated discovery, scoped access, end-to-end protection, bounds on relay work, and a storage and metadata policy. The test exercises transport interruption and reconstruction of a collector from public bytes. It does not exercise a wallet process crash, persistent mailbox recovery, private keys on a device, or human approval screens.

## Reproduce

From this directory on the repository's pinned Rust toolchain:

```sh
cargo test --release --locked -- --nocapture
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Committed fixtures suffice for replay. To regenerate them, check out PRISM_v2 at the submodule revision `87fad7da5f12f79112521677f4f5530672afe9ba`, and set REF_SOURCE and REF_BUILD to its absolute source and scratch build directories:

```sh
cmake -S "$REF_SOURCE" -B "$REF_BUILD" -DCMAKE_BUILD_TYPE=Release -DPRISM_BUILD_TYPE=ref -DGMP_LIBRARY=MINI -DENABLE_SIGN=ON
cmake --build "$REF_BUILD" --target prism_signature_lvl1 prism_signature_lvl3 prism_signature_lvl5
PRISM_REF_SOURCE="$REF_SOURCE" PRISM_REF_BUILD="$REF_BUILD" sh ../../scripts/export_s5_vectors.sh backend
cargo run --release --locked --example generate -- "$REF_BUILD"
```

All seeds are public repeated-byte test values 1, 2 and 3. Reproduction intentionally writes the fixture files. No participant wallet or live output is involved.

For the onion replay, use a lab-owned copy of Tor and tor-gencert on PATH, and Chutney revision `3cd543f00555ae970fcb7dc3f2fd1f0d91c3c2d2`. Set CHUTNEY_PATH to its absolute checkout and CHUTNEY_DATA_DIR to a fresh directory. Use an isolated host or namespace with Chutney's default ports free. From the experiment directory:

```sh
export CHUTNEY_PATH CHUTNEY_DATA_DIR
export ECHO=echo CHUTNEY_CONFIG_PHASES=1 CHUTNEY_LAUNCH_PHASES=2
cargo build --release --locked --example onion
inbox=$(mktemp -d)
"$CHUTNEY_PATH/tools/bootstrap-network.sh" hs-v3-min
target/release/examples/onion serve &
mailbox_pid=$!
trap 'kill "$mailbox_pid"; "$CHUTNEY_PATH/chutney" stop "$CHUTNEY_PATH/networks/hs-v3-min"' EXIT
target/release/examples/onion run "$CHUTNEY_DATA_DIR/nodes/007h/hidden_service/hostname" 9006 "$inbox"
```

## Consequences for the next experiment

The payment digest is stable when a signer changes, while two authorized pairs produce different witness bytes for the same spend. A chain design must specify transaction identity, conflict handling, input ownership, fee/weight effects and what the recipient tracks. The fixed 2-of-3 format holds the witness size constant within each set. It does not model change, multiple inputs, chain expiry enforcement, balance, nullifiers or membership proofs.

The experiment demonstrates an independent-signature quorum. It reveals two signer positions and keys and performs two verifications. A single-key threshold signature and a private proof of quorum remain separate comparisons. For receiving, the most important missing experiment is a payer who retains every send-side secret: show that an offline group can receive, the payer cannot spend, and two payments cannot be publicly linked through the group policy. For delivery, replace the preconfigured shared meeting point with a private invitation and test a returning wallet after the proposer disconnects. These experiments can proceed entirely in shekyl-prism.
