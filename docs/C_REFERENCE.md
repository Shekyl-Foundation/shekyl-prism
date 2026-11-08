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

A match against that verifier is a match against salt-PRISM, including those
quirks. It is not conformance to `docs/VERIFIER_SPEC.md`.
