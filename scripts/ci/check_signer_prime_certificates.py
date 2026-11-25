#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Verify the fixed signer primes and the next-prime rules of SIGNER_SPEC §1.

Run with Python 3.8+ from any directory. The PARI ECPP and Lucas N+1
certificates are inputs. All proof arithmetic uses Python integers.

A Pratt leaf is prime, composite, or an unfinished search. Only a
composite is factored. A projective residue is a unit, zero, or a
rejected proper factor of the modulus.
"""
import enum
import json
import sys
from functools import lru_cache
from math import gcd, isqrt, prod
from pathlib import Path

SIGNER = Path(__file__).resolve().parents[2] / "prism-vectors" / "data" / "signer"
CERTS = SIGNER / "certificates"
SETS = ("p248", "p324", "p376", "p500", "p664")

# PARI primecert: the chain's final prime q satisfies q ≤ 2^64. 2^64 is
# even, so a proven leaf is strictly below this power.
LEAF_LIMIT = 1 << 64
# Trial division through the square root is a complete proof below this
# bound. The square root is 2^10. At and above the bound, Pratt is the proof.
LEAF_TRIAL_LIMIT = 1 << 20
# Exclusive end of the Pratt base search. A prime with no witness in
# range is unfinished: the search stops, and the integer is not composite.
LEAF_WITNESS_LIMIT = 4096
# Constants c in the Pollard polynomial x^2 + c. A cycle that returns n
# itself tries the next offset.
POLLARD_OFFSETS = (1, 2, 3, 5, 7, 11)
# Gap compositeness: trial division below this bound, then Miller-Rabin
# bases in [2, GAP_MILLER_BASE_LIMIT). No witness means the gap check fails.
GAP_TRIAL_LIMIT = 1000
GAP_MILLER_BASE_LIMIT = 64
# SIGNER_SPEC 1.4.4: the prime list ends at 1237.
SMALL_PRIME_LAST = 1237
# Identity of the short-Weierstrass group in Jacobian coordinates.
POINT_AT_INFINITY = (0, 1, 0)


class LeafKind(enum.Enum):
    """A decided leaf. An unfinished search is not a kind: it raises."""

    PRIME = "prime"
    COMPOSITE = "composite"


class LeafSearchIncomplete(ValueError):
    """The search did not decide. The integer is not shown composite, and it is not split."""


class Residue(enum.Enum):
    """A residue mod N that the group law may use. A proper shared factor is neither."""

    ZERO = "zero"
    UNIT = "unit"


class ImproperResidue(ValueError):
    """A residue shares a proper factor with the modulus."""


def jacobi_pure(a: int, n: int) -> int:
    if n <= 0 or n % 2 == 0:
        raise ValueError("jacobi symbol requires odd positive n")
    a %= n
    result = 1
    while a:
        while a % 2 == 0:
            a //= 2
            if n % 8 in (3, 5):
                result = -result
        a, n = n, a
        if a % 4 == 3 and n % 4 == 3:
            result = -result
        a %= n
    return result if n == 1 else 0


def is_prime_trial(n: int) -> bool:
    if n < 2:
        return False
    if n in (2, 3):
        return True
    if n % 2 == 0 or n % 3 == 0:
        return False
    r = isqrt(n)
    f = 5
    while f <= r:
        if n % f == 0 or n % (f + 2) == 0:
            return False
        f += 6
    return True


def residue(value: int, modulus: int) -> Residue:
    """`ZERO` or `UNIT`. A proper shared factor raises: it is not a point."""
    reduced = value % modulus
    if reduced == 0:
        return Residue.ZERO
    if gcd(reduced, modulus) != 1:
        raise ImproperResidue("residue shares a proper factor with the modulus")
    return Residue.UNIT


def pollard_divisor(n: int) -> int:
    """A proper divisor of composite `n`.

    Called only after `classify_leaf` has shown `n` composite. A prime has
    no proper divisor; that raises here, and is not reported as a prime.
    """
    if n % 2 == 0:
        if n == 2:
            raise ValueError(f"no proper divisor of {n}")
        return 2
    for offset in POLLARD_OFFSETS:
        x = 2
        y = 2
        divisor = 1
        while divisor == 1:
            x = (x * x + offset) % n
            y = (y * y + offset) % n
            y = (y * y + offset) % n
            divisor = gcd(abs(x - y), n)
        if 1 < divisor < n:
            return divisor
    raise ValueError(f"no proper divisor of {n}")


def pratt_quotient(order: int, prime: int) -> int:
    """`order / prime`, exact. Floor division would invent an exponent."""
    if prime <= 1 or order % prime != 0:
        raise ValueError(f"prime factor {prime} does not divide the group order {order}")
    return order // prime


def distinct_prime_factors(n: int) -> set:
    """Distinct prime factors of `n`. A piece the search cannot decide raises."""
    if n < 2:
        raise ValueError(f"no prime factorization of {n}")
    pending = [n]
    factors = set()
    while pending:
        piece = pending.pop()
        if classify_leaf(piece) is LeafKind.PRIME:
            factors.add(piece)
            continue
        divisor = pollard_divisor(piece)
        pending.append(divisor)
        pending.append(piece // divisor)
    return factors


@lru_cache(maxsize=None)
def classify_leaf(n: int, witness_limit: int = LEAF_WITNESS_LIMIT) -> LeafKind:
    """`PRIME` or `COMPOSITE` for an integer below `LEAF_LIMIT`.

    `LeafSearchIncomplete` means the search stopped. Callers that factor
    must not treat that as composite: only `COMPOSITE` is split.
    `witness_limit` is the exclusive end of the Pratt base search. The
    factorization of `n - 1` always uses `LEAF_WITNESS_LIMIT`.
    """
    if witness_limit < 2:
        raise LeafSearchIncomplete(
            f"Pratt witness limit {witness_limit} leaves no base to test"
        )
    if n < 2 or n >= LEAF_LIMIT:
        power = LEAF_LIMIT.bit_length() - 1
        raise LeafSearchIncomplete(f"{n} is outside the Pratt leaf range [2, 2^{power})")
    if n < LEAF_TRIAL_LIMIT:
        if is_prime_trial(n):
            return LeafKind.PRIME
        return LeafKind.COMPOSITE
    if pow(2, n - 1, n) != 1:
        return LeafKind.COMPOSITE

    factors = distinct_prime_factors(n - 1)
    quotients = tuple(pratt_quotient(n - 1, prime) for prime in factors)
    for base in range(2, witness_limit):
        if pow(base, n - 1, n) != 1:
            return LeafKind.COMPOSITE
        if all(pow(base, quotient, n) != 1 for quotient in quotients):
            return LeafKind.PRIME
    raise LeafSearchIncomplete(f"no Pratt witness below {witness_limit} for {n}")


def require_proven_prime(n: int) -> None:
    """Raise unless `n` is a proven prime below `LEAF_LIMIT`."""
    if classify_leaf(n) is not LeafKind.PRIME:
        raise ValueError(f"{n} is composite")


def exceeds_atkin_bound(q: int, n: int) -> bool:
    """True iff q > (N**(1/4) + 1)**2, with only integer arithmetic."""
    if q <= 1 or n < 0:
        return False
    # (sqrt(q) - 1)^4 > N  <=>  A > 4(q+1)sqrt(q), A = q^2 + 6q + 1 - N
    a = q * q + 6 * q + 1 - n
    if a <= 0:
        return False
    return a * a > 16 * (q + 1) * (q + 1) * q


def ec_double(point, a: int, n: int):
    x, y, z = point
    # Z is decided first. Y = 0 is 2-torsion only on a point whose Z is a unit.
    if residue(z, n) is Residue.ZERO or residue(y, n) is Residue.ZERO:
        return POINT_AT_INFINITY
    if residue(2 * y, n) is not Residue.UNIT:
        raise ImproperResidue("noninvertible doubling denominator")
    xx = (x * x) % n
    yy = (y * y) % n
    yyyy = (yy * yy) % n
    s = (4 * x * yy) % n
    zz = (z * z) % n
    m = (3 * xx + (a * zz * zz)) % n
    x3 = (m * m - 2 * s) % n
    y3 = (m * (s - x3) - 8 * yyyy) % n
    z3 = (2 * y * z) % n
    return (x3, y3, z3)


def ec_add(p, q, a: int, n: int):
    x1, y1, z1 = p
    x2, y2, z2 = q
    if residue(z1, n) is Residue.ZERO:
        return q
    if residue(z2, n) is Residue.ZERO:
        return p
    z1z1 = (z1 * z1) % n
    z2z2 = (z2 * z2) % n
    u1 = (x1 * z2z2) % n
    u2 = (x2 * z1z1) % n
    s1 = (y1 * z2z2 * z2) % n
    s2 = (y2 * z1z1 * z1) % n
    if (u1 - u2) % n == 0:
        if (s1 - s2) % n == 0:
            return ec_double(p, a, n)
        if (s1 + s2) % n != 0:
            raise ValueError("mixed addition and doubling modulo factors of N")
        return POINT_AT_INFINITY
    h = (u2 - u1) % n
    r = (s2 - s1) % n
    if residue(h, n) is not Residue.UNIT:
        raise ImproperResidue("noninvertible addition denominator")
    h2 = (h * h) % n
    h3 = (h2 * h) % n
    x3 = (r * r - h3 - 2 * u1 * h2) % n
    y3 = (r * (u1 * h2 - x3) - s1 * h3) % n
    z3 = (z1 * z2 * h) % n
    return (x3, y3, z3)


def ec_mul(k: int, point, a: int, n: int):
    result = POINT_AT_INFINITY
    addend = point
    while k:
        if k & 1:
            result = ec_add(result, addend, a, n)
        addend = ec_double(addend, a, n)
        k >>= 1
    return result


def verify_ecpp(n, steps):
    # PARI 2.17, primecert: each link proves N given the primality of q.
    if not steps:
        raise ValueError("empty ECPP chain")
    for index, step in enumerate(steps, 1):
        t, s, a = (int(step[key]) for key in ("t", "s", "a"))
        x, y = map(int, step["P"])
        if int(step["N"]) != n or n <= 3 or n % 2 == 0:
            raise ValueError(f"ECPP step {index}: wrong or inadmissible modulus")
        m = n + 1 - t
        if t * t >= 4 * n or s <= 0 or m % s:
            raise ValueError(f"ECPP step {index}: invalid trace or cofactor")
        q = m // s
        if not exceeds_atkin_bound(q, n):
            raise ValueError(f"ECPP step {index}: q below the required order bound")
        b = (y * y - x**3 - a * x) % n
        if gcd(4 * a**3 + 27 * b**2, n) != 1:
            raise ValueError(f"ECPP step {index}: singular curve modulo a factor of N")

        # residue() rejects a Z that vanishes on only some factors of N,
        # so every finite intermediate is a unit on all of them.
        point = ec_mul(s, (x % n, y % n, 1), a, n)
        closed = ec_mul(q, point, a, n)
        finite = residue(point[2], n) is Residue.UNIT
        if not finite or residue(closed[2], n) is not Residue.ZERO:
            raise ValueError(f"ECPP step {index}: [s]P does not have order q")
        n = q
    require_proven_prime(n)


def lucas_uv(k: int, p: int, q: int, n: int):
    """Return (U_k, V_k, Q^k) modulo n, with n odd."""
    if k == 0:
        return 0, 2 % n, 1
    # Binary method, starting from (U1, V1) = (1, P).
    # We work with (U, V, Q^e) for the current exponent e.
    # Double: U_{2e} = U V, V_{2e} = V^2 - 2 Q^e, Q^{2e} = (Q^e)^2
    inv2 = pow(2, -1, n)
    u, v, qe = 1, p % n, q % n
    # We have exponent 1 already. Walk the remaining bits of k.
    for bit in range(k.bit_length() - 2, -1, -1):
        # double
        u = (u * v) % n
        v = (v * v - 2 * qe) % n
        qe = (qe * qe) % n
        if (k >> bit) & 1:
            # (U,V) = (U,V)_{2e} -> (U,V)_{2e+1}
            # U_{i+1} = (P U_i + V_i)/2
            # V_{i+1} = (D U_i + P V_i)/2, D = P^2 - 4Q
            d = (p * p - 4 * q) % n
            u1 = ((p * u + v) * inv2) % n
            v1 = ((d * u + p * v) * inv2) % n
            u, v = u1, v1
            qe = (qe * q) % n
    return u, v, qe


def verify_lucas(n, cert):
    p, q = int(cert["P"]), int(cert["Q"])
    d = p * p - 4 * q
    if n <= 2 or n % 2 == 0 or int(cert["D"]) != d:
        raise ValueError("invalid Lucas modulus or discriminant")
    if gcd(q, n) != 1 or jacobi_pure(d, n) != -1:
        raise ValueError("invalid Lucas parameters")
    factors = [(int(r), int(e)) for r, e in cert["np1_factors"].items()]
    if any(exponent < 1 for _, exponent in factors):
        raise ValueError("invalid prime power in N+1")
    for prime, _ in factors:
        require_proven_prime(prime)
    if prod(r**e for r, e in factors) != n + 1:
        raise ValueError("factorization does not give N+1")
    if lucas_uv(n + 1, p, q, n)[0] != 0:
        raise ValueError("U(N+1) is nonzero")
    for r, _ in factors:
        if gcd(lucas_uv((n + 1) // r, p, q, n)[0], n) != 1:
            raise ValueError(f"Lucas rank condition failed for factor {r}")


TRIAL_PRIMES = [p for p in range(2, GAP_TRIAL_LIMIT) if is_prime_trial(p)]


def has_composite_witness(n):
    if n < 4:
        return False
    for p in TRIAL_PRIMES:
        if n % p == 0:
            return n != p
    # A failed strong probable-prime test is a proof of compositeness.
    d = n - 1
    s = 0
    while d % 2 == 0:
        d //= 2
        s += 1
    for a in range(2, GAP_MILLER_BASE_LIMIT):
        x = pow(a, d, n)
        if x in (1, n - 1):
            continue
        for _ in range(s - 1):
            x = x * x % n
            if x == n - 1:
                break
        else:
            return True
    return False


def verify_gap(n, power):
    base = 1 << power
    if n <= base:
        raise ValueError(f"claimed next prime is not above 2^{power}")
    for candidate in range(base + 1, n):
        if not has_composite_witness(candidate):
            raise ValueError(f"no composite witness for 2^{power} + {candidate - base}")


def read_rows(filename):
    rows = {}
    for line in (SIGNER / filename).read_text().splitlines():
        if line and not line.startswith("#"):
            key, value = line.split(" ", 1)
            if key in rows:
                raise ValueError(f"{filename}: duplicate {key}")
            rows[key] = value
    return rows


def certificate(name, method):
    path = CERTS / f"{name}.{method}.json"
    cert = json.loads(path.read_text())
    if cert["id"] != name:
        raise ValueError(f"{path.name}: certificate ID differs")
    return cert


def check_ecpp(name, expected=None, power=None):
    cert = certificate(name, "ecpp")
    n = int(cert["N"])
    if expected is not None and n != expected:
        raise ValueError(f"{name}: certificate differs from the signer constant")
    verify_ecpp(n, cert["steps"])
    if power is not None:
        verify_gap(n, power)
        print(f"{name}: ECPP verified, {n - (1 << power) - 1} gap witnesses")
    else:
        print(f"{name}: ECPP verified")


def verify_all():
    oracle = read_rows("oracle_c_reference.txt")
    shared = read_rows("shared.txt")
    small = [2] + [p for p in range(5, SMALL_PRIME_LAST + 1, 4) if is_prime_trial(p)]
    if list(map(int, shared["small_primes"].split())) != small or int(shared["small_primes_length"]) != len(small):
        raise ValueError("shared small-prime list differs from trial division")
    complementary = [p for p in range(3, SMALL_PRIME_LAST, 4) if is_prime_trial(p)]
    bad_product = prod(complementary)
    norms = {}
    for name in SETS:
        rows = read_rows(f"{name}.txt")
        e = int(rows["e"])
        if rows["set"] != name or name != f"p{e}":
            raise ValueError(f"{name}: wrong parameter set")
        p = (int(rows["f"]) << e) - 1
        check_ecpp(f"field-{name}", p)
        cert = certificate(f"field-{name}", "np1")
        if int(cert["N"]) != p:
            raise ValueError(f"{name}: Lucas certificate differs from the field modulus")
        verify_lucas(p, cert)
        print(f"field-{name}: Lucas N+1 verified")

        expected = None
        if name in ("p248", "p376", "p500"):
            expected = int(oracle[f"{name}.prime_cofactor"])
            if int(oracle[f"{name}.bad_primes_product"]) != bad_product:
                raise ValueError(f"{name}: complementary prime product differs")
        check_ecpp(f"cofactor-{name}", expected, p.bit_length())

        power = 2 * ((e + 127) // 128 * 128)
        norm = int.from_bytes(bytes.fromhex(rows["secret_norm"]), "little")
        if power in norms and norms[power] != norm:
            raise ValueError(f"{name}: sets with the same width disagree on secret_norm")
        norms[power] = norm
    for power, norm in norms.items():
        check_ecpp(f"secret-norm-2p{power}", norm, power)
    print(f"Verified {2 * len(SETS) + len(norms)} fixed primes, both small-prime lists and all next-prime gaps.")


if __name__ == "__main__":
    try:
        verify_all()
    except (ValueError, OSError, KeyError, TypeError) as err:
        sys.exit(f"FAIL: {err}")
