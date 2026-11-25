# Fixed signer prime certificates

Run `python3 scripts/ci/check_signer_prime_certificates.py` from the repository root.
Python 3.8 or later verifies all five parameter sets using its standard library.

| Files | Values checked |
| --- | --- |
| `field-*.ecpp.json` and `field-*.np1.json` | The modulus computed from each signer's `e` and `f` rows. The Lucas proof uses the complete factorization of the modulus plus one. |
| `secret-norm-*.ecpp.json` | The little-endian `secret_norm` in each signer file. P324 and P376 use the same proof. |
| `cofactor-*.ecpp.json` | Item 19 of the signer specification at each set, also compared with the pinned C oracle at P248, P376 and P500. |

For each norm and cofactor, the checker derives the starting power of two from the set and proves every integer between it and the certified value composite. It checks the two small-prime lists by trial division.

The JSON files are byte-for-byte copies of the research proofs generated with PARI/GP 2.17.4. The generator, inventory, native PARI files and Primo exports are available in the [pinned research tree](https://github.com/Shekyl-Foundation/shekyl-prism/tree/894d5041240b5cded19b7d75d3aeca7ff68f741b/research/prime-certificates). The ECPP chain format and order bound are specified in the [PARI manual](https://pari.math.u-bordeaux.fr/dochtml/html-stable/Arithmetic_functions.html#primecert).
