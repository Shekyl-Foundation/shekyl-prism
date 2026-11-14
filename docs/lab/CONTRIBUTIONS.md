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
