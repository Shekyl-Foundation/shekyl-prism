# Contributions and AI disclosure

Status: living record, started 2026-11-13. Ruled by Rick 2026-11-13: AI participation is disclosed in full.

## People

- **Rick Dawson**, lead architect, Shekyl Foundation. Set the goals, made every design ruling (dated in `NOTEBOOK.md`), managed, guided, and coordinated the AI systems, merged every pull request, and conducted all the huiman interaction, including with the PRISM authors.

## AI systems

- **Design partner and reviewer (Claude, Anthropic, in a chat session).** Read the code and papers, proposed designs, wrote one consolidated brief per pull request, reviewed each pull request against its head commit, and drafted these notes. It had read access to the repository and no write access. Model identifier as configured for the session that covered 2026-11-10 to 2026-11-13: `claude-fable-5-2`; the model serving a given turn may have differed. Earlier sessions: to be filled in from their records.
- **Coding agents (on the Foundation's development machine).** Implemented each brief, ran the Sage model and all experiments, wrote the repository documents, and opened the pull requests. More than one tool did this. What the machine's records show, as read on 2026-11-13:
  - Cursor's agent: the co-author trailer on commit `8f1f8bb` (2026-11-07).
  - OpenAI Codex CLI, model `gpt-6.1-sol`: sessions in this repository started on 2026-11-12 and last active on 2026-11-13, covering PR #15 up to `9972661` (the implementation checkpoint `6caebb6` and the Sage evidence runner). What its later activity produced is not established here.
  - Claude Code, model `claude-fable-5-2`: from 2026-11-12, the rest of PR #15 (comparison drivers, experiments E1 to E6, records, the comparison report, the security-argument rewrite, review fixes; the branch's commits from `b98925f` to `3770c82`) and the check of these notes against the tree.
  - Which tool wrote each commit from 2026-11-07 to 2026-11-11, and the two commits Rick added to PR #15 (`bf163f3`, `cb9f6ec`): to be filled in by Rick.
- **Other tools.** Commit `8f1f8bb`, pushed before PR #1, carries a Cursor co-author trailer; it is the only commit on `dev` that carries one. Pull-request review bots (CodeQL, Copilot) raised findings on #15; each is recorded with its verdict in that pull request.
- **Coding agent (Grok, xAI, in Grok Build).** On 2026-11-17, the review of PR #29 at `4d05f0f` and the representation fix that followed: a prepared key keeps its bytes and its basis index, and the coefficient is decoded from those bytes. On the same day, the structure review of PR #31 and `b0dee9c`: a set identifier is the built set whose prime is the pair read, found by searching `Set::ALL`, and an accept list is a `BTreeSet` of those identifiers ordered by the prime. On 2026-11-26, the measurement restructure of PR #49: the Stage 2c corpus, the release probe, and the integer-divide predicate each have one owner; timing contrasts each fixture with its class reference; the stack scan covers every reduction.

## Other code bases read

Oracles for values, not sources of structure (the Round 0 axiom). What was learned from each is recorded here so that it is not presented as ours.

- **`sqisign-rs`** (Anchorage Labs, <https://github.com/anchorageoss/sqisign-rs>, MIT or Apache-2.0), read at `7a4a01b` on 2026-11-21 for signer stage 1a. Learned from its generator `tools/gen-precomp`: that the starting basis of `E0[2^e]` is the output of Algorithm B.1 of the SQIsign specification of 2026-09-01, which led to the check that the pinned C reference's basis follows the same rule; the layout of the action matrices (columns as images); and the construction of the two halved generators' actions through a half of each basis point taken over `F_{p⁴}`. Its checked-in constants at `p324`, `p500` and `p664` are the comparison values `docs/SIGNER_SPEC.md` names. No code of it is in this repository. Read again at the same commit on 2026-11-22 for signer stage 2a (`docs/SIGNER_SPEC.md` 2.1.2). Learned from its integer type `Ibz<N>` and its generator: that one limb count for each parameter set serves every layer of a signer; the rule `5·⌈log₂ p / 64⌉` and that rule's stated shortfall at `p248` and `p376`; and a statement of the term that decides the width, the Gram matrix of the ideal to be reduced (`2·(nb + 3) + log₂ p + 3` bits for a norm of `nb` bits). Our bound for the same term was derived before that comment was read and agrees with it to five bits. Our width, our overflow reporting and our split into a public and a secret type are not taken from it: its integers cut a result that does not fit and report nothing.

- **Crates the integer layer is built on** (signer stage 2a, 2026-11-22). `zeroize` 1.8.2 (the RustCrypto developers, MIT or Apache-2.0) writes the zeros of the wipe on drop; `subtle` 2.6.1 (Isis Lovecruft and Henry de Valence, BSD-3-Clause) gives the selection of a limb by a bit and its optimization barrier. In the tests only: `num-bigint` 0.4.6 with `num-integer` and `num-traits` (the Rust Num developers, MIT or Apache-2.0), the arbitrary-precision oracle. The division is Knuth's algorithm D (*The Art of Computer Programming*, volume 2, 4.3.1), written from the book's description; the first pair of operands the vectors use to reach its add-back step is the one in the test cases of `divmnu`, the companion code of Warren's *Hacker's Delight*, scaled to 64-bit limbs and to each width; the others are found by a search (`scripts/int_vectors.py`).

## What git shows and does not show

Every commit is authored under Rick's identity. Repository rule 90 keeps AI attribution trailers out of commit messages, so git history does not show which commits an AI agent wrote. Most of them were. The notebook's "Who" line on each entry is the record.

## Division of intellectual work

To be kept current. As of 2026-11-13:

| Item | Proposed by | Implemented by | Checked by |
| --- | --- | --- | --- |
| Carry the challenge search variable in the signature | Rick (ruling 11-07) | coding agent | reviewer |
| Chain shortened to `e′ = e − 2` | reviewer, ruled by Rick 11-09 | coding agent | reviewer |
| Keep full-order `2^e` signature points | reviewer finding, ruled by Rick 11-10 | coding agent | Sage model |
| Step 8 on `E_vk[2^e]` with both components | reviewer | coding agent | Sage model, negatives |
| Canonical Montgomery model of `E_sig` | reviewer (probe on real vectors) | coding agent | model item 13 |
| Verification-key export fault (determinant −1 shear) | coding agent | coding agent | reviewer read the diff |
| Explicit Weil pairing check | reviewer | coding agent | Sage known answers |
| Closed-form symplectic matrices in `s` | reviewer | coding agent | chain verdicts against the library completion on 73 signatures and the negatives; null points against Sage; reviewer's 6,000 random checks (not committed) |
| Cornacchia fault in the pinned C reference | coding agent | coding agent (exporter patch) | reviewer read the routine |
| Collision-replay cost and the salt | coding agent wrote the analysis (#14); reviewer measured it and proposed the salt; ruled by Rick 11-12 | coding agent | experiments E1 to E6 |
| Hedged salt derivation | reviewer, on Rick's question 11-12 | coding agent | reviewer |
| Operation-count metric as the machine-independent cost signal | reviewer, ruled by Rick 11-08; brief by reviewer 11-13 | coding agent (Claude Code) | CI on x86_64 and aarch64 |
| Verifier cost: squaring, inversion-free Miller loop, shared chain inversion, scalar-multiplication reuse | reviewer named the four targets from a profile and the refusals to preserve | coding agent (Claude Code) chose the constructions (fraction-carrying Jacobian loop, one batch per chain, doubling tables per basis point) | the replaced routines kept as references; 2,712 negative verdicts pinned; Sage differential |
| A prepared key stores its bytes and its basis index; the coefficient is decoded from the bytes | review of PR #29 at `4d05f0f` | coding agent (Grok, xAI, in Grok Build) | unit test `the_coefficient_is_the_bytes_and_the_index_is_that_search`; `scripts/op_counts.sh` |
| What each signer constant is, which the signer needs, and the rule of the starting basis (`docs/SIGNER_SPEC.md` section 1) | reviewer (brief of 2026-11-21); the basis rule found through `sqisign-rs` | coding agent (Claude Code) | scratch computations, not committed; the tests are stage 1b's |
| The generator of the signer's constants: the basis rule, the actions through half points over `F_{p⁴}`, the next-prime test, the toy primes; and that the first case of the rule's last step is unreachable (`scripts/signer_constants.py`, `docs/SIGNER_SPEC.md` 1.4.1, 1.8) | reviewer (brief of 2026-11-21: tool, placement, shape, the wargame); the halving through the three square roots follows `sqisign-rs`'s construction, credited above | coding agent (Claude Code) | both oracles at every set either has; the engine's own test in Rust; the reviewer's independent basis run |
