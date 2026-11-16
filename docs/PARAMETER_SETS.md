# Parameter sets

Status: current as of 2026-11-15. Ruled by Rick on 2026-11-14.

A parameter set is a prime `p = f·2^e − 1`. There is one algorithm, and
each set is that algorithm over one prime. A set is named by its exponent:
`p248`, `p376`, `p500`. In the code a set is a type (`P248`, `P376`,
`P500` in `prism-field`). Its name, exponent and cofactor are constants
of that type. Its salt length and its search bound are `e` rounded up
to a multiple of 128, in bytes and in candidates: `ParameterSet::WIDTH_BITS`.

**No code reads this file.** In particular no code reads the category
column below. The category a prime is assessed at is an assessment, and it
moves with research: the same 500-bit prime was a level V prime in ePrint
2026/443 and is a level III prime in the SQIsign specification of
2026-09-01. Rick's ruling, in his words: "The NIST level is an assessment
that moves with research. The same 500-bit prime was level V and is now
level III. The level is documentation, not a mathematical fact, and it is
not coded. The levels collide; the math does not."

## The sets

Five primes are ruled, and all five are in the code: three with vectors
and a signer, and `p324` and `p664` on the verifier side only. Sizes are
in bytes.

| Set | Prime | `e` | `f` | Field element | Public key | Signature | Salt | In the code |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `p248` | `5·2^248 − 1` | 248 | 5 | 32 | 64 | 352 | 32 | yes |
| `p324` | `3·2^324 − 1` | 324 | 3 | 41 | 82 | 458 | 48 | yes, verifier side; accept path unproven (O14) |
| `p376` | `65·2^376 − 1` | 376 | 65 | 48 | 96 | 528 | 48 | yes |
| `p500` | `27·2^500 − 1` | 500 | 27 | 64 | 128 | 704 | 64 | yes |
| `p664` | `17·2^664 − 1` | 664 | 17 | 84 | 168 | 936 | 96 | yes, verifier side; accept path unproven (O14) |

- A public key is one element of `F_{p²}`, two field elements. A
  signature is five elements of `F_{p²}` and the salt.
- The salt is `e` rounded up to a multiple of 128 bits. The three sets
  with vectors already had those lengths; the rule is what fixes `p324`
  and `p664` (ruled 2026-11-14). `width_bits` is that rounding.
- Every size in the table is pinned by a test in `prism-field`.
- A `p324` field element is 41 bytes, the first that is not a whole
  number of 8-byte limbs; its top byte is at most `0x2f`. A `p664` field
  element is 84 bytes in eleven limbs, the first above eight; its top
  byte is at most `0x10`.
- The search bound is the same rounding, counted in candidates: 256, 384,
  384, 512 and 768.
  The public-key search is the one search with a failure estimate, and
  that estimate lives on `prism_verify::search_bound`. The rational basis
  and the starting basis share the limit and have no estimate.
- Two integers the verifier computes with are sized by the set, as types
  the set names. The challenge degree's primality test runs on
  `ParameterSet::Degree`: 1,024 bits at the four sets up to `p500`, and
  1,344 bits at `p664`, whose degree needs 1,324. The matrices of the
  accept path have entries modulo `2^e` in `ParameterSet::TorsionLimbs`:
  eight limbs at the four sets up to `p500`, eleven at `p664`. The four
  older sets keep the widths they were built with; narrowing them to what
  each needs is a later change.

## What is and is not shown at `p324` and `p664`

There is no signer at `p324` or at `p664`, so there is no honest
signature, no vector and no committed label for either. **Nothing shows
that `verify` accepts anything at `p324` or at `p664`.** That is ledger
obligation O14, and it stays open until a signer exists for these primes.

The two sets are tested the same way, by the same code: one Sage script
(`scripts/sage/no_corpus_fixture.sage`) and one set of tests
(`prism-verify/tests/no_corpus_*.rs`), each run at both. What is
checked, without a signature:

- field arithmetic in `F_p` and `F_{p²}` against Sage, on edge values and
  on 10,000 random rows;
- decoding: exactly the integers below `p`, in exactly 41 bytes at
  `p324` and 84 at `p664`;
- on eight supersingular curves from Sage: which points are of full
  order (a point missing the cofactor's prime, 3 or 17, or one factor of
  2, is not), the basis index, the `2^e` basis and its Weil pairing;
- the pairing against the affine loop it replaced, on two of those
  curves, one from each branch of the basis search (curves 1 and 7 at
  `p324`, curves 1 and 3 at `p664`);
- the challenge scalar against an independent script (321 bits at
  `p324`, 661 at `p664`), and the primality test at each size, at the
  width the verifier uses for that set, against `sympy`;
- every refusal that can be reached from public data: `Length`,
  `Canonical`, `Origin`, `Singular`, `Sign`, `Model`, `Composite`,
  `Kernel`, `Pairing` and `Codomain`.

The pairing check does not need the secret key. It asks only that the
pairing of the response points be the inverse of the pairing of the
key's basis, and anyone can meet that: take the key's own curve as the
response curve and answer with the verifier's own basis `(P, Q)` as
`(P, −Q)` or as `(P, P − Q)`. Neither has an isogeny behind it. The
first is refused as `Kernel` in the first chain. The second is refused
as `Codomain`, and to return that the verifier has built the kernel,
run both theta chains and compared the codomains. So that code runs at
both sets, on a response it must refuse, and refuses it, with the same
two verdicts as at the three sets with vectors. The field operations of
both refusals are in
[benchmarks/op-counts.txt](benchmarks/op-counts.txt). For the `Codomain`
case they are 295,378 extension multiplications and squarings and 72
inversions at `p324`, and 632,495 and 76 at `p664`.
That is the cost of a refusal, not of an accepted verification, which
does not exist at these primes.

Both refusals are committed bytes at all five sets
(`prism-verify/testdata/identity_refusals.tsv`), and the `Codomain` one
is timed. On the Raspberry Pi 4 floor device at `351deb4`, one thread,
under the resident node's load:

Table: Pi 4, milliseconds. The middle column is a refusal, not an accepted verification; the last is **Derived** where no accept exists.

| Set | Accept, measured | `Codomain` refusal, measured | Accept, **Derived** from the refusal |
| --- | ---: | ---: | ---: |
| `p248` | 89.77 | 86.10 | |
| `p324` | none | 192.76 | 201 to 203 |
| `p376` | 232.39 | 221.38 | |
| `p500` | 520.50 | 493.34 | |
| `p664` | none | 1,171.48 | 1,221 to 1,236 |

The derived column divides the refusal by the refusal-to-accept time
ratio measured at the three sets that have both (0.948 to 0.959), and
assumes the ratio at the other two lies in that range. The record, the
x86_64 figures and the sender's cost of producing such a response are in
[BENCHMARKING.md](BENCHMARKING.md), "The refusal after the pairing
check, at all five sets".

Not reached at either set: `Basis`, `Image`, and acceptance. `Image` is
compared only after the codomains agree, and no response buildable from
public data is known to get them to.

## What each prime is currently assessed at

Each row says only what its source says.

| Set | Assessed at | Source | Date of the source |
| --- | --- | --- | --- |
| `p248` | No current assessment. NIST category I in ePrint 2026/443, Table 2, which predates ePrint 2026/1486. The SQIsign specification of 2026-09-01 does not list this prime. | ePrint 2026/443 | 2026 |
| `p324` | NIST category I | SQIsign specification 3.0, §5.2 | 2026-09-01 |
| `p376` | No current assessment as a selected prime. NIST category III in ePrint 2026/443, Table 2, which predates ePrint 2026/1486. The SQIsign specification of 2026-09-01 lists it among the further primes it gives for category I and does not select it. | ePrint 2026/443; SQIsign specification 3.0, appendix on other primes | 2026; 2026-09-01 |
| `p500` | NIST category III. It was category V in ePrint 2026/443, Table 2. | SQIsign specification 3.0, §5.2 | 2026-09-01 |
| `p664` | NIST category V | SQIsign specification 3.0, §5.2 | 2026-09-01 |

These are the sources' assessments of SQIsign and of salt-PRISM as
published. This repository's construction is modified (`e′ = e − 2`, the
carried salt, the canonical encoding), and its own security argument is
`SECURITY_ARGUMENT.md`, whose obligations are open. Nothing here is a
security claim for this repository.

## Which set Shekyl uses

`p500` (ruled 2026-11-14). Which sets a Shekyl-facing verifier accepts is
a policy list with that one entry; the other sets are for tests, research
and comparison. The list is not yet enforced in the type system: today
`prism_verify::verify` runs at any implemented set.

## Names in older records

Records written before 2026-11-14 name the sets by the level they were
assessed at then. The mapping is fixed:

| In records before 2026-11-14 | Set |
| --- | --- |
| `NIST-I`, level I, `lvl1` | `p248` |
| `NIST-III`, level III, `lvl3` | `p376` |
| `NIST-V`, level V, `lvl5` | `p500` |

Those strings are also data. Committed vectors carry `level NIST-I` and so
on, file names carry `lvl1`, and the research signer absorbs the string
into the streams its public test seeds expand to. The string is not a
constant of the prime. `Committed` in `prism-verify` is implemented for
the three sets that have a corpus, and `Committed::LABEL` is the string.
A prime with no corpus, such as `p324`, does not implement it, and the
run-time value `CommittedSet` cannot be such a set: asking for its label
is the error `NoCorpus`, not a placeholder. No vector, fixture or
record file was renamed or edited when the sets were renamed.
