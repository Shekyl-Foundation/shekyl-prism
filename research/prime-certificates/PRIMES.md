# Fixed primes of the constants generator

Research inventory for Shekyl PRISM. PRISM is the signature scheme of
ePrint 2026/443 (Basso, Borin, Castryck, Corte-Real Santos, Invernizzi,
Leroux, Maino, Vercauteren, and Wesolowski); `shekyl-prism` implements
it. This directory does not change the signer, the signature format, or
any parameter set.

Sets covered: P248, P324, P376, P500, and P664 only. P363 and P384 are
not Shekyl parameter sets.

The values below are taken from `dev` at
[`26ce141`](https://github.com/Shekyl-Foundation/shekyl-prism/commit/26ce1410286728e21120fc98da710b6995a04920),
from `prism-vectors/data/signer/` and from the same rules in
`scripts/signer_constants.py`. Machine-readable copy: [`primes.json`](primes.json).

Ledger O19 records that each field modulus and each secret norm is only a
Baillie–PSW probable prime today, and that the nightly SymPy check
(`scripts/ci/check_signer_primes.py`) is the same kind of test. This
experiment pins independently checkable certificates instead.

## 1. Field primes `p = f · 2^e − 1` (five)

These are the field moduli. `p + 1 = f · 2^e` is completely factored at
every set, so an `N+1` (Lucas) proof is short, as O19 already noted.

| Set | Formula | Bits | `p+1` factors |
|-----|---------|------|----------------|
| P248 | `5 · 2^248 − 1` | 251 | `2^248 · 5` |
| P324 | `3 · 2^324 − 1` | 326 | `2^324 · 3` |
| P376 | `65 · 2^376 − 1` | 383 | `2^376 · 5 · 13` |
| P500 | `27 · 2^500 − 1` | 505 | `2^500 · 3^3` |
| P664 | `17 · 2^664 − 1` | 669 | `2^664 · 17` |

Each `p` is `3 (mod 4)`. Decimal and hex values, and a SHA-256 of the
decimal representation, are in `primes.json`.

## 2. Secret-ideal starting norms (item 33, ruling R1)

The next prime after `2^(2 · WIDTH_BITS)`, where
`WIDTH_BITS = ⌈e / 128⌉ · 128`. The generator finds it with its own
Baillie–PSW test. P324 and P376 share `WIDTH_BITS = 384`, so they share
one prime: there are five listings and four unique values.

| Set | Power of two | Offset | Bits |
|-----|----------------|--------|------|
| P248 | `2^512` | `+75` | 513 |
| P324, P376 | `2^768` | `+183` | 769 |
| P500 | `2^1024` | `+643` | 1025 |
| P664 | `2^1536` | `+75` | 1537 |

## 3. Prime cofactors (item 19)

The next prime after `2^{bit_length(p)}`. The generator compares the
C-reference oracle at P248, P376 and P500 against this rule. P324 and
P664 have no C-reference oracle; the same rule still produces a fixed
prime. Item 19 is read by two C-reference benchmarks and is not on the
native signing path (`SIGNER_SPEC` 1.2, ledger O30). It is still a fixed
prime the generator relies on, so it is certified here.

| Set | Power of two | Offset | Bits | In C oracle |
|-----|----------------|--------|------|-------------|
| P248 | `2^251` | `+65` | 252 | yes |
| P324 | `2^326` | `+249` | 327 | no |
| P376 | `2^383` | `+369` | 384 | yes |
| P500 | `2^505` | `+51` | 506 | yes |
| P664 | `2^669` | `+1067` | 670 | no |

## 4. Tiny primes, proven by trial division

No ECPP file is stored for these. `verify.py` checks the lists against
the committed data and proves each entry by trial division up to its
integer square root.

- **`small_primes`** (items 17 and 23): `2` and the first 100 primes that
  are `1 (mod 4)`. There are 101 values, from `2` to `1237`, in
  `prism-vectors/data/signer/shared.txt`.
- **Bad primes, `3 (mod 4)`:** the 102 primes that are `3 (mod 4)` and
  strictly less than `1237` (last is `1231`). The generator uses their
  product as the complement of `small_primes` when it checks the
  C-reference's `bad_primes_product`. They are not a row of the shipped
  `pXXX.txt` files.

## What is not in this list

Runtime probable primes — challenge degrees, reduced-ideal norms, and
other values found while signing — are not fixed constants and are not
certified here. Toy primes used only by `signer_constants.py --selftest`
are also out of scope.
