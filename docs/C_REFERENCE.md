# C reference

Submodule: `external/PRISM_v2`, commit
`87fad7da5f12f79112521677f4f5530672afe9ba` of
<https://github.com/mariascrs/PRISM_v2>. Apache-2.0. Some third-party files
in that tree are LGPL-3.0; this workspace does not link them.

The README of that repository calls the build salt-PRISM. Parameter sets
`lvl1`, `lvl3`, and `lvl5` are present (`p5248`, `p65376`, `p27500`), matching
Table 2's three primes.

`scripts/check_c_reference.py` fails if the pinned tree no longer contains
the verifier quirks the spec records:

- `src/verification/ref/lvlx/common.c` draws a salt when `*salt == 0xFFFFFFFF`
- the salt is absorbed as `sizeof(uint32_t)` bytes
- primality is `ibz_probab_prime(challenge_degree, 20)`
- `src/verification/ref/lvlx/verify.c` calls
  `theta_chain_compute_and_eval_verify`

`ec_curve_to_basis_2f_from_hint` in `src/ec/ref/lvlx/basis.c` carries a
recovery path the script does not pin, because `prism-verify` reaches the
same answer without it: a hint of zero means the key-generation search
needed an index of 128 or more, and that function restarts the search there.
Both of its paths name the smallest qualifying index, so the Rust verifier —
which carries no hint byte — derives that index in a single search from 1.
`docs/VERIFIER_SPEC.md` records the one bound that remains and its
probability.

`theta_chain_compute_and_eval_verify` is the known-answer source for
Algorithm 3's chain, compared at the function level: the same kernel
inputs, the codomain, and the evaluated images, with the chain length as
a parameter. The salt-PRISM wrapper around that function is not the
encoding this workspace checks.

A match against that verifier is a match against salt-PRISM, including those
quirks. It is not conformance to `docs/VERIFIER_SPEC.md`.

`prism-verify/testdata/c_basis_hints.txt` is one key per level from that pin.
`scripts/export_c_basis_hints.py` rebuilds the tree with mini-gmp and writes
the file again from a frozen seed (`SEED_BASE` in the script; the file names
the seed each level accepted). The same seed writes the same bytes. CI diffs
the file, on a little-endian host: the reference absorbs a salt in native
byte order, so a big-endian run is not a reproduction of this file.
