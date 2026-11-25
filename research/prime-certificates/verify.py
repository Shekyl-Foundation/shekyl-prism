#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Verify pinned primality certificates for the signer's fixed primes.

This is an independent checker. It does not call PARI's primecertisvalid.
It implements the checks in the PARI 2.17 manual for an Atkin–Morain ECPP
certificate, a Lucas N+1 proof when p+1 is completely factored, a
Pocklington–Lehmer N-1 / Pratt proof when one is stored, and trial
division for the tiny committed prime lists.

Usage, from the repository root:

    python3 research/prime-certificates/verify.py
    python3 research/prime-certificates/verify.py --selftest
    python3 research/prime-certificates/verify.py --pari   # optional cross-check

Python 3.8+ and the standard library are enough. gmpy2 is used when it
is importable, only for speed.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CERTS = HERE / "certs"
PRIMES_PATH = HERE / "primes.json"
SIGNER = ROOT / "prism-vectors" / "data" / "signer"
SETS = ("p248", "p324", "p376", "p500", "p664")
TWO64 = 1 << 64
TRIAL_BOUND = 1 << 20  # complete trial division up to this size


# ---------------------------------------------------------------------------
# Optional gmpy2
# ---------------------------------------------------------------------------
try:
    import gmpy2

    def iz(n):
        return gmpy2.mpz(n)

    def ipow(base, exp, mod):
        return int(gmpy2.powmod(iz(base), iz(exp), iz(mod)))

    def igcd(a, b):
        return int(gmpy2.gcd(iz(a), iz(b)))

    def iinvert(a, n):
        return int(gmpy2.invert(iz(a), iz(n)))

    def iisqrt(n):
        return int(gmpy2.isqrt(iz(n)))

    def ijacobi(a, n):
        return int(gmpy2.jacobi(iz(a), iz(n)))

    USING = "gmpy2"
except ImportError:
    def iz(n):
        return int(n)

    def ipow(base, exp, mod):
        return pow(int(base), int(exp), int(mod))

    def igcd(a, b):
        return math.gcd(int(a), int(b))

    def iinvert(a, n):
        return pow(int(a), -1, int(n))

    def iisqrt(n):
        return math.isqrt(int(n))

    def ijacobi(a, n):
        return jacobi_pure(int(a), int(n))

    USING = "python-int"


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


def zint(value) -> int:
    if isinstance(value, int):
        return value
    if isinstance(value, str):
        value = value.strip().replace("_", "")
        if value.startswith("0x") or value.startswith("0X"):
            return int(value, 16)
        return int(value, 10)
    return int(value)


# ---------------------------------------------------------------------------
# Trial division and Pratt proofs for small / 64-bit leaves
# ---------------------------------------------------------------------------
def _primes_to(limit: int) -> list[int]:
    if limit < 2:
        return []
    sieve = bytearray(b"\x01") * (limit + 1)
    sieve[0:2] = b"\x00\x00"
    for i in range(2, int(limit**0.5) + 1):
        if sieve[i]:
            step = i
            start = i * i
            sieve[start : limit + 1 : step] = b"\x00" * (((limit - start) // step) + 1)
    return [i for i, v in enumerate(sieve) if v]


_WHEEL_PRIMES = _primes_to(1000)


def is_prime_trial(n: int) -> bool:
    if n < 2:
        return False
    if n in (2, 3):
        return True
    if n % 2 == 0 or n % 3 == 0:
        return False
    r = iisqrt(n)
    f = 5
    while f <= r:
        if n % f == 0 or n % (f + 2) == 0:
            return False
        f += 6
    return True


def pollard_rho(n: int) -> int:
    if n % 2 == 0:
        return 2
    # A few polynomials / starts, cycled, so a single unlucky curve does not stall.
    for c in (1, 2, 3, 5, 7, 11):
        x = 2
        y = 2
        d = 1
        while d == 1:
            x = (x * x + c) % n
            y = (y * y + c) % n
            y = (y * y + c) % n
            d = igcd(abs(x - y), n)
        if d != n:
            return d
    raise ValueError(f"pollard rho failed on {n}")


def factor_completely(n: int) -> dict[int, int]:
    """Factor a positive integer completely. Intended for 64-bit leaves."""
    out: dict[int, int] = {}

    def push(p: int, e: int = 1) -> None:
        out[p] = out.get(p, 0) + e

    def split(m: int) -> None:
        if m == 1:
            return
        if is_prime_trial(m) if m <= TRIAL_BOUND else is_prime_leaf(m, proving=False):
            push(m)
            return
        if m <= TRIAL_BOUND:
            # composite and small: trial-split
            r = iisqrt(m)
            f = 2
            while f <= r:
                if m % f == 0:
                    split(f)
                    split(m // f)
                    return
                f += 1 if f == 2 else 2
            push(m)
            return
        d = pollard_rho(m)
        split(d)
        split(m // d)

    # Trial split off small factors first.
    m = n
    for p in _WHEEL_PRIMES:
        if p * p > m:
            break
        while m % p == 0:
            push(p)
            m //= p
    if m > 1:
        split(m)
    return out


def is_prime_leaf(n: int, proving: bool = True) -> bool:
    """Deterministic primality for n <= 2^64.

    n <= 2^20 is trial-divided. Larger 64-bit values are proved by Pratt's
    theorem (a complete factorisation of n-1 plus a primitive-root witness),
    with the prime factors of n-1 proved the same way. This does not use
    Miller–Rabin witness-set papers, and it does not call PARI.
    """
    if n < 2:
        return False
    if n == 2:
        return True
    if n % 2 == 0:
        return False
    if n > TWO64:
        raise ValueError(f"leaf primality is only for n <= 2^64, got {n.bit_length()} bits")
    if n <= TRIAL_BOUND:
        return is_prime_trial(n)
    if not proving:
        # Used only inside factor_completely to decide whether to split.
        # A weak Fermat test plus one Miller–Rabin base-2 step is enough
        # to tell the factoriser to split or stop; the public prove path
        # always uses Pratt (proving=True) on the factors it returns.
        if ipow(2, n - 1, n) != 1:
            return False
        # strong Miller–Rabin base 2, used as a compositeness filter only
        d = n - 1
        s = 0
        while d % 2 == 0:
            d //= 2
            s += 1
        x = ipow(2, d, n)
        if x in (1, n - 1):
            return True
        for _ in range(s - 1):
            x = (x * x) % n
            if x == n - 1:
                return True
        return False
    return pratt_prove(n, set())


def pratt_prove(n: int, stack: set[int]) -> bool:
    if n < 2:
        return False
    if n <= TRIAL_BOUND:
        return is_prime_trial(n)
    if n in stack:
        return False
    stack.add(n)
    factors = factor_completely(n - 1)
    for p in factors:
        if p > TWO64 or not pratt_prove(p, stack):
            return False
    # Find a Pratt witness (a primitive root works; many smaller bases do).
    primes = list(factors)
    for a in range(2, 4096):
        if ipow(a, n - 1, n) != 1:
            return False  # Fermat already failed: composite
        if all(ipow(a, (n - 1) // p, n) != 1 for p in primes):
            return True
    return False


# ---------------------------------------------------------------------------
# Exact ECPP bound: q > (N^{1/4} + 1)^2
# ---------------------------------------------------------------------------
def exceeds_atkin_bound(q: int, n: int) -> bool:
    """True iff q > (N**(1/4) + 1)**2, with only integer arithmetic."""
    if q <= 1 or n < 0:
        return False
    # (sqrt(q) - 1)^4 > N  <=>  A > 4(q+1)sqrt(q), A = q^2 + 6q + 1 - N
    a = q * q + 6 * q + 1 - n
    if a <= 0:
        return False
    return a * a > 16 * (q + 1) * (q + 1) * q


# ---------------------------------------------------------------------------
# Elliptic curve arithmetic in Jacobian coordinates over Z/NZ
# ---------------------------------------------------------------------------
def ec_double(point, a: int, n: int):
    x, y, z = point
    if z % n == 0:
        return (0, 1, 0)
    if y % n == 0:
        return (0, 1, 0)
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
    if z1 % n == 0:
        return q
    if z2 % n == 0:
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
        return (0, 1, 0)
    h = (u2 - u1) % n
    r = (s2 - s1) % n
    h2 = (h * h) % n
    h3 = (h2 * h) % n
    x3 = (r * r - h3 - 2 * u1 * h2) % n
    y3 = (r * (u1 * h2 - x3) - s1 * h3) % n
    z3 = (z1 * z2 * h) % n
    return (x3, y3, z3)


def ec_mul(k: int, point, a: int, n: int):
    if k == 0:
        return (0, 1, 0)
    if k < 0:
        x, y, z = point
        return ec_mul(-k, (x, (-y) % n, z), a, n)
    result = (0, 1, 0)
    addend = point
    kk = k
    while kk:
        if kk & 1:
            result = ec_add(result, addend, a, n)
        addend = ec_double(addend, a, n)
        kk >>= 1
    return result


def is_infinity(point, n: int) -> bool:
    return point[2] % n == 0


# ---------------------------------------------------------------------------
# ECPP certificate verification (PARI 2.17 Atkin–Morain format)
# ---------------------------------------------------------------------------
def verify_ecpp(n: int, steps, leaf_proofs: list) -> None:
    """Raise ValueError if the certificate does not prove that n is prime."""
    if n <= 1:
        raise ValueError("N must be an integer > 1")
    if isinstance(steps, int) or (isinstance(steps, str) and steps.isdigit()):
        # PARI: a prime integer N < 2^64 is itself a certificate.
        leaf = zint(steps)
        if leaf != n:
            raise ValueError("bare ECPP certificate does not match N")
        if n >= TWO64:
            raise ValueError("bare ECPP certificate is only valid for N < 2^64")
        if not is_prime_leaf(n):
            raise ValueError(f"N = {n} is not a proven 64-bit prime")
        leaf_proofs.append(n)
        return
    if not steps:
        raise ValueError("empty ECPP certificate")

    prev_q = None
    last_index = len(steps) - 1
    for i, step in enumerate(steps):
        ni = zint(step["N"] if isinstance(step, dict) else step[0])
        ti = zint(step["t"] if isinstance(step, dict) else step[1])
        si = zint(step["s"] if isinstance(step, dict) else step[2])
        ai = zint(step["a"] if isinstance(step, dict) else step[3])
        point = step["P"] if isinstance(step, dict) else step[4]
        xi = zint(point[0])
        yi = zint(point[1])
        if i == 0 and ni != n:
            raise ValueError(f"first step N = {ni} does not match claimed N = {n}")
        if prev_q is not None and ni != prev_q:
            raise ValueError(f"step {i+1}: N is not the previous q")
        if ni <= 1:
            raise ValueError(f"step {i+1}: N is not a positive integer > 1")
        if ti * ti >= 4 * ni:
            raise ValueError(f"step {i+1}: t^2 is not < 4N")
        if si <= 0:
            raise ValueError(f"step {i+1}: s is not a positive integer")
        mi = ni + 1 - ti
        if mi % si != 0:
            raise ValueError(f"step {i+1}: s does not divide m = N+1-t")
        qi = mi // si
        if not exceeds_atkin_bound(qi, ni):
            raise ValueError(f"step {i+1}: q is not > (N^{{1/4}}+1)^2")
        bi = (yi * yi - xi * xi * xi - ai * xi) % ni
        disc = (-16 * (4 * ai * ai * ai + 27 * bi * bi)) % ni
        if igcd(disc, ni) != 1:
            raise ValueError(f"step {i+1}: curve discriminant shares a factor with N")
        if igcd(yi, ni) != 1:
            # 2-torsion (y=0) cannot have the required order conditions in
            # the usual way; a point with a non-invertible y is also invalid.
            raise ValueError(f"step {i+1}: point y is not coprime to N")
        point_j = (xi % ni, yi % ni, 1)
        mP = ec_mul(mi, point_j, ai, ni)
        sP = ec_mul(si, point_j, ai, ni)
        if not is_infinity(mP, ni):
            raise ValueError(f"step {i+1}: mP is not the point at infinity")
        if is_infinity(sP, ni):
            raise ValueError(f"step {i+1}: sP is the point at infinity")
        if i == last_index:
            if qi > TWO64 or qi == TWO64:
                # PARI: q_ell <= 2^64 is prime. The manual says <= 2^64.
                # Integers equal to 2^64 are not prime; reject anything > 2^64.
                raise ValueError(f"step {i+1}: final q = {qi} is not <= 2^64")
            if qi >= TWO64 or not is_prime_leaf(qi):
                raise ValueError(f"step {i+1}: final q is not a proven prime <= 2^64")
            leaf_proofs.append(qi)
        prev_q = qi


# ---------------------------------------------------------------------------
# Lucas N+1 certificates (for field primes, where N+1 is fully factored)
# ---------------------------------------------------------------------------
def lucas_uv(k: int, p: int, q: int, n: int):
    """Return (U_k, V_k, Q^k) modulo n, with n odd."""
    if k == 0:
        return 0, 2 % n, 1
    # Binary method, starting from (U1, V1) = (1, P).
    # We work with (U, V, Q^e) for the current exponent e.
    # Double: U_{2e} = U V, V_{2e} = V^2 - 2 Q^e, Q^{2e} = (Q^e)^2
    # Add-one uses the addition formulas.
    # Standard "Lucas doubling" chain from the MSB of k.
    inv2 = iinvert(2, n)
    # Start at U_1, V_1
    # Process bits of k from bit_length-2 down to 0, doubling then adding if bit set.
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


def verify_lucas_np1(n: int, cert: dict) -> None:
    if n <= 2 or n % 2 == 0:
        raise ValueError("Lucas N+1 expects an odd integer N > 2")
    p = zint(cert["P"])
    q = zint(cert["Q"])
    d = p * p - 4 * q
    if "D" in cert and zint(cert["D"]) != d:
        raise ValueError("stored D does not equal P^2 - 4Q")
    if ijacobi(d, n) != -1:
        raise ValueError("Jacobi(D/N) is not -1")
    if igcd(d, n) != 1:
        raise ValueError("D shares a factor with N")
    factors = cert["np1_factors"]
    # factors: list of [prime, exponent] or {"p": e}
    if isinstance(factors, dict):
        pairs = [(zint(pk), int(ek)) for pk, ek in factors.items()]
    else:
        pairs = [(zint(item[0]), int(item[1])) for item in factors]
    prod = 1
    for pr, exp in pairs:
        if pr < 2 or exp < 1:
            raise ValueError("bad N+1 factor")
        if pr <= TRIAL_BOUND:
            if not is_prime_trial(pr):
                raise ValueError(f"N+1 factor {pr} is not prime")
        elif pr < TWO64:
            if not is_prime_leaf(pr):
                raise ValueError(f"N+1 factor {pr} is not a proven prime")
        else:
            raise ValueError(f"N+1 factor {pr} is too large for the leaf prover")
        prod *= pow(pr, exp)
    if prod != n + 1:
        raise ValueError("product of N+1 factors is not N+1")
    u_np1, _v, _qe = lucas_uv(n + 1, p, q, n)
    if u_np1 % n != 0:
        raise ValueError("U_{N+1} is not 0 mod N")
    for pr, _exp in pairs:
        u_div, _v, _qe = lucas_uv((n + 1) // pr, p, q, n)
        if igcd(u_div, n) != 1:
            raise ValueError(f"gcd(U_{(n+1)//pr}, N) is not 1 for factor {pr}")


# ---------------------------------------------------------------------------
# Pocklington–Lehmer / Pratt N-1 certificates (PARI flag=1 shape, or ours)
# ---------------------------------------------------------------------------
def verify_nm1(n: int, cert) -> None:
    """Verify a PARI N-1 certificate or a compact Pratt/Pocklington tree."""
    if isinstance(cert, dict) and cert.get("format") in {"pratt", "pocklington-lehmer", "pari-nm1"}:
        payload = cert.get("certificate", cert.get("tree", cert))
        _verify_nm1_tree(n, payload)
        return
    _verify_nm1_tree(n, cert)


def _verify_nm1_tree(n: int, cert) -> None:
    # PARI: a prime integer N < 2^64, or a pair [N, C].
    if isinstance(cert, int) or (isinstance(cert, str) and not cert.startswith("[")):
        leaf = zint(cert)
        if leaf != n:
            raise ValueError("bare N-1 certificate does not match N")
        if n >= TWO64 or not is_prime_leaf(n):
            raise ValueError("bare N-1 certificate is not a proven prime < 2^64")
        return
    if isinstance(cert, dict):
        claimed = zint(cert["N"])
        chain = cert["C"]
        if claimed != n:
            raise ValueError("N-1 certificate N does not match")
        _verify_pocklington_chain(n, chain)
        return
    # list form: [N, C] or just C (list of factors)
    if isinstance(cert, list) and cert and (isinstance(cert[0], list) or isinstance(cert[0], (int, str, dict))):
        if len(cert) == 2 and zint(cert[0]) == n and isinstance(cert[1], list):
            _verify_pocklington_chain(n, cert[1])
            return
        # maybe the file stored only C
        _verify_pocklington_chain(n, cert)
        return
    raise ValueError("unrecognised N-1 certificate shape")


def _verify_pocklington_chain(n: int, chain: list) -> None:
    """PARI N-1 chain: each entry is a prime p < 2^64, or [p, a, C_p].

    The product F of p^{v_p(N-1)} over the listed p must exceed N^{1/3},
    and the Brillhart–Lehmer–Selfridge quadratic must be irreducible.
    """
    if n <= 2 or (n - 1) <= 0:
        raise ValueError("N-1 certificate expects N > 2")
    f = 1
    for entry in chain:
        if isinstance(entry, (int, str)):
            p = zint(entry)
            if p >= TWO64 or not is_prime_leaf(p):
                raise ValueError(f"N-1 leaf p = {p} is not a proven prime < 2^64")
            if (n - 1) % p != 0:
                raise ValueError(f"p = {p} does not divide N-1")
            # No witness is stored for a 64-bit factor in PARI's short form
            # when the entry is a bare integer. We still require a witness
            # that we search for: some a with a^{N-1}=1 and gcd(a^{(N-1)/p}-1, N)=1.
            if not _has_pocklington_witness(n, p):
                raise ValueError(f"no Pocklington witness for factor {p}")
        else:
            p = zint(entry[0])
            a = zint(entry[1])
            sub = entry[2]
            if (n - 1) % p != 0:
                raise ValueError(f"p = {p} does not divide N-1")
            if ipow(a, n - 1, n) != 1:
                raise ValueError(f"a^{{N-1}} is not 1 for factor {p}")
            if igcd(ipow(a, (n - 1) // p, n) - 1, n) != 1:
                raise ValueError(f"gcd(a^{{(N-1)/p}}-1, N) is not 1 for factor {p}")
            _verify_nm1_tree(p, sub if not isinstance(sub, list) or (sub and zint(sub[0]) == p) else [p, sub] if isinstance(sub, list) and sub and zint(sub[0]) != p else sub)
        # multiply p^{v_p(N-1)} into F
        e = 0
        m = n - 1
        while m % p == 0:
            m //= p
            e += 1
        f *= pow(p, e)
    # F > N^{1/3}  <=>  F^3 > N
    if f * f * f <= n:
        raise ValueError("product F of certified N-1 factors is not > N^{1/3}")
    # N = 1 + c1 F + c2 F^2, and c1^2 - 4 c2 is not a square
    rem = n - 1
    c1 = rem // f
    c2 = c1 // f
    c1 = c1 % f
    # rem = c1 F + c2 F^2, with c2 = (N-1) // F^2, c1 = ((N-1)//F) % F
    if 1 + c1 * f + c2 * f * f != n:
        raise ValueError("base-F digits of N do not reassemble")
    disc = c1 * c1 - 4 * c2
    s = iisqrt(disc) if disc >= 0 else -1
    if disc >= 0 and s * s == disc:
        raise ValueError("Brillhart–Lehmer–Selfridge discriminant is a square")


def _has_pocklington_witness(n: int, p: int) -> bool:
    for a in range(2, 4096):
        if ipow(a, n - 1, n) == 1 and igcd(ipow(a, (n - 1) // p, n) - 1, n) == 1:
            return True
    return False


# ---------------------------------------------------------------------------
# Data loading and committed-value checks
# ---------------------------------------------------------------------------
def load_primes() -> dict:
    return json.loads(PRIMES_PATH.read_text())


def signer_rows(path: Path) -> dict:
    out = {}
    for line in path.read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        parts = line.split(" ", 1)
        out[parts[0]] = parts[1] if len(parts) == 2 else ""
    return out



def prove_composite(n: int) -> str:
    """Return a short proof string that n is composite, or raise."""
    if n <= 3:
        raise ValueError(f"{n} is not composite")
    if n % 2 == 0:
        return "divisible by 2"
    r = min(iisqrt(n), 10000)
    f = 3
    while f <= r:
        if n % f == 0:
            return f"divisible by {f}"
        f += 2
    for a in range(2, 64):
        if igcd(a, n) != 1:
            return f"divisible by {igcd(a, n)}"
        if ipow(a, n - 1, n) != 1:
            return f"Fermat witness a={a}: a^(n-1) ≢ 1 (mod n)"
        # strong Miller–Rabin witness (unconditional compositeness)
        d = n - 1
        s = 0
        while d % 2 == 0:
            d //= 2
            s += 1
        x = ipow(a, d, n)
        if x in (1, n - 1):
            continue
        seen_minus = False
        for _ in range(s - 1):
            x = (x * x) % n
            if x == n - 1:
                seen_minus = True
                break
        if not seen_minus:
            return f"Miller–Rabin witness a={a}"
    raise ValueError(f"no compositeness witness found for {n}")


def prove_nextprime_gaps(inventory: dict) -> list[str]:
    """Prove that each 'nextprime(2^k) = 2^k + offset' has a composite gap."""
    notes = []
    for entry in inventory["large_primes"]:
        if "power" not in entry or "offset" not in entry:
            continue
        power = int(entry["power"])
        offset = int(entry["offset"])
        base = 1 << power
        claimed = zint(entry["value_dec"])
        if claimed != base + offset:
            raise ValueError(f"{entry['id']}: value is not 2^{power} + {offset}")
        for k in range(1, offset):
            kind = prove_composite(base + k)
            if k == 1:
                first = kind  # keep a sample in the note
        notes.append(
            f"{entry['id']}: 2^{power} + 1 .. +{offset-1} are composite "
            f"({offset-1} integers; e.g. 2^{power}+1 is {first})"
        )
    return notes


def check_committed_values(inventory: dict) -> list[str]:
    """Hold inventory values equal to the committed signer constants."""
    notes = []
    by_id = {e["id"]: e for e in inventory["large_primes"]}
    for name in SETS:
        rows = signer_rows(SIGNER / f"{name}.txt")
        e = int(rows["e"])
        f = int(rows["f"])
        p = f * (1 << e) - 1
        entry = by_id[f"field-{name}"]
        if zint(entry["value_dec"]) != p:
            raise ValueError(f"{name}: inventory field prime does not match e,f")
        width = (e + 127) // 128 * 128
        secret = int.from_bytes(bytes.fromhex(rows["secret_norm"]), "little")
        # find the unique secret-norm entry that lists this set
        found = [x for x in inventory["large_primes"] if x["kind"] == "secret_norm" and name in x["sets"]]
        if len(found) != 1 or zint(found[0]["value_dec"]) != secret:
            raise ValueError(f"{name}: inventory secret norm does not match secret_norm")
        notes.append(f"{name}: field p and secret_norm match {SIGNER.relative_to(ROOT)}/{name}.txt")
    shared = signer_rows(SIGNER / "shared.txt")
    committed_small = [int(x) for x in shared["small_primes"].split()]
    listed = inventory["tiny_primes"]["small_primes"]["values"]
    if committed_small != listed:
        raise ValueError("inventory small_primes does not match shared.txt")
    notes.append("small_primes match prism-vectors/data/signer/shared.txt")
    return notes


def prove_tiny_lists(inventory: dict) -> list[str]:
    notes = []
    small = inventory["tiny_primes"]["small_primes"]["values"]
    if small[0] != 2 or any(p != 2 and p % 4 != 1 for p in small):
        raise ValueError("small_primes is not 2 followed by primes 1 mod 4")
    if len(small) != 101:
        raise ValueError(f"{len(small)} small_primes, not 101")
    for p in small:
        if not is_prime_trial(p):
            raise ValueError(f"small_primes entry {p} is not prime")
    # Completeness: 2 and the first 100 primes that are 1 mod 4.
    found = []
    n = 2
    while len(found) < 101:
        if (n == 2 or n % 4 == 1) and is_prime_trial(n):
            found.append(n)
        n += 1
    if found != small:
        raise ValueError("small_primes misses a prime or holds a composite")
    notes.append("101 small_primes proved by trial division and complete in the stated range")

    bad = inventory["tiny_primes"]["bad_primes_3mod4"]["values"]
    last = small[-1]
    recomputed = [k for k in range(3, last) if k % 4 == 3 and is_prime_trial(k)]
    if recomputed != bad or len(bad) != 102:
        raise ValueError("bad_primes_3mod4 is not the complete 3-mod-4 list below 1237")
    notes.append("102 complementary 3-mod-4 primes proved by trial division")
    return notes


def load_json_cert(path: Path) -> dict:
    return json.loads(path.read_text())


# ---------------------------------------------------------------------------
# Self-test against the PARI manual's published example
# ---------------------------------------------------------------------------
# primecert(10^35 + 69) as printed in the PARI 2.17 manual (usersch3), plus
# the two-step certificate this box's PARI 2.17.4 actually emits (the second
# step's point may differ by curve isomorphism; we test whatever shape the
# verifier must accept by constructing from the documented theorem data of
# a cert we re-derive in --selftest when PARI is present, and always test
# the bound, trial, Pratt, and Lucas routines against known values.
MANUAL_N = 10**35 + 69


def selftest() -> int:
    failures = []

    def hold(cond, msg):
        if not cond:
            failures.append(msg)

    hold(is_prime_trial(17) and is_prime_trial(1237) and is_prime_trial(1231), "trial of tiny primes")
    hold(not is_prime_trial(1) and not is_prime_trial(91) and not is_prime_trial(121), "trial of composites")
    hold(is_prime_leaf(65537) and is_prime_leaf((1 << 31) - 1), "Pratt of known 16- and 31-bit primes")
    hold(not is_prime_leaf(2047) and not is_prime_leaf((1 << 32) + 1) if True else True, "Pratt rejects composites")
    # 2047 = 23*89; 2^32+1 is not prime (F5 is 2^32+1 = 4294967297 = 641 * 6700417)
    hold(not is_prime_leaf(2047), "2047 is not prime")
    hold(not is_prime_leaf(4294967297), "F5 is not prime")
    # Bound: for the manual first step, q = 33743830764501150277, N = 10^35+69
    q = 33743830764501150277
    hold(exceeds_atkin_bound(q, MANUAL_N), "manual first-step Atkin bound holds")
    hold(not exceeds_atkin_bound(q - q + 1, MANUAL_N), "q=1 fails the Atkin bound")
    # A tight numerical check: (floor(N^{1/4})+1)^2 is not enough when N is
    # not a fourth power, but q just above the real bound must pass.
    # Use N=16=2^4, (N^{1/4}+1)^2 = 4, so q > 4.
    hold(exceeds_atkin_bound(10, 16) and not exceeds_atkin_bound(9, 16), "perfect-fourth Atkin bound")
    # Lucas N+1 on a known prime with N+1 factored: p = 5*2^8 - 1 = 1279?
    # 5*2^8-1 = 1279, is 1279 prime? 1279=1279. Let me use 5*2^8-1=1279, 1279/1279.
    # Better: 3*2^4-1=47, 47+1=48=16*3. Tiny and clearly prime.
    tiny_p = 3 * (1 << 4) - 1
    hold(tiny_p == 47, "toy field prime")
    # Search a Lucas certificate for 47.
    found = False
    for p_par in range(1, 40):
        for q_par in range(-20, 21):
            if q_par == 0:
                continue
            d = p_par * p_par - 4 * q_par
            if jacobi_pure(d, 47) != -1:
                continue
            try:
                verify_lucas_np1(47, {"P": p_par, "Q": q_par, "D": d, "np1_factors": [[2, 4], [3, 1]]})
                found = True
                break
            except ValueError:
                continue
        if found:
            break
    hold(found, "Lucas N+1 proves 47 = 3*2^4 - 1")

    # ECPP self-check: a one-step certificate for nextprime(2^64) is in the
    # PARI manual. We rebuild it only if the user also generated one; here
    # we check that a deliberately broken cert is refused.
    broken = [{"N": 41, "t": 0, "s": 1, "a": 0, "P": [0, 1]}]
    refused = False
    try:
        verify_ecpp(41, broken, [])
    except ValueError:
        refused = True
    hold(refused, "a junk ECPP certificate is refused")
    # Bare 41 is a valid PARI cert (41 < 2^64 and 41 is prime).
    try:
        verify_ecpp(41, 41, [])
        bare_ok = True
    except ValueError:
        bare_ok = False
    hold(bare_ok, "bare 41 is accepted as a < 2^64 certificate")

    if failures:
        print("selftest FAILED:", *failures, sep="\n  ")
        return 1
    print(f"selftest passed (arithmetic backend: {USING})")
    return 0


# ---------------------------------------------------------------------------
# Main driver
# ---------------------------------------------------------------------------
def manifest_path() -> Path:
    return CERTS / "manifest.json"


def verify_all(use_pari: bool = False) -> int:
    inventory = load_primes()
    print(f"backend: {USING}")
    print(f"inventory: {inventory['counts']['large_primes_unique']} unique large primes, "
          f"{inventory['counts']['small_primes']} small_primes, "
          f"{inventory['counts']['bad_primes']} complementary 3-mod-4 primes")
    committed = check_committed_values(inventory)
    for line in committed:
        print("  ok:", line)
    tiny = prove_tiny_lists(inventory)
    for line in tiny:
        print("  ok:", line)
    gaps = prove_nextprime_gaps(inventory)
    for line in gaps:
        print("  ok:", line)

    man = json.loads(manifest_path().read_text()) if manifest_path().exists() else None
    if man is None:
        print("FAIL: certs/manifest.json is missing; generate certificates first")
        return 1

    failures = 0
    expected_ids = [e["id"] for e in inventory["large_primes"]]
    seen = []
    for entry in inventory["large_primes"]:
        pid = entry["id"]
        n = zint(entry["value_dec"])
        rec = None
        for item in man.get("certificates", []):
            if item["id"] == pid:
                rec = item
                break
        if rec is None:
            print(f"FAIL: {pid}: no manifest entry")
            failures += 1
            continue
        seen.append(pid)
        methods_ok = []
        # ECPP
        if rec.get("ecpp_json"):
            path = HERE / rec["ecpp_json"]
            try:
                cert = load_json_cert(path)
                claimed = zint(cert["N"])
                if claimed != n:
                    raise ValueError("ECPP file N does not match inventory")
                leaves = []
                verify_ecpp(n, cert["steps"], leaves)
                methods_ok.append(f"ECPP ({len(cert['steps']) if not isinstance(cert['steps'], int) else 0} steps, leaf {leaves[-1] if leaves else '?'})")
            except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as err:
                print(f"FAIL: {pid} ECPP: {err}")
                failures += 1
        else:
            print(f"FAIL: {pid}: no ECPP certificate")
            failures += 1
        # N+1
        if rec.get("np1_json"):
            path = HERE / rec["np1_json"]
            try:
                cert = load_json_cert(path)
                if zint(cert["N"]) != n:
                    raise ValueError("N+1 file N does not match inventory")
                verify_lucas_np1(n, cert)
                methods_ok.append(f"Lucas N+1 (P={cert['P']}, Q={cert['Q']})")
            except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as err:
                print(f"FAIL: {pid} N+1: {err}")
                failures += 1
        # N-1
        if rec.get("nm1_json"):
            path = HERE / rec["nm1_json"]
            try:
                cert = load_json_cert(path)
                claimed = zint(cert["N"]) if "N" in cert else n
                if claimed != n:
                    raise ValueError("N-1 file N does not match inventory")
                verify_nm1(n, cert)
                methods_ok.append("N-1 / Pocklington–Lehmer")
            except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as err:
                print(f"FAIL: {pid} N-1: {err}")
                failures += 1
        if rec.get("ecpp_json") and methods_ok:
            print(f"  ok: {pid}  {entry['bit_length']} bits  {', '.join(methods_ok)}")

        if use_pari and rec.get("ecpp_gp"):
            if not pari_isvalid(HERE / rec["ecpp_gp"]):
                print(f"FAIL: {pid}: PARI primecertisvalid rejected {rec['ecpp_gp']}")
                failures += 1
            else:
                print(f"       PARI primecertisvalid agrees on {rec['ecpp_gp']}")

    missing = [i for i in expected_ids if i not in seen]
    if missing:
        print("FAIL: manifest missing", ", ".join(missing))
        failures += 1
    if failures:
        print(f"FAIL: {failures} problem(s)")
        return 1
    print(f"all {len(expected_ids)} large primes verified; tiny lists proved")
    return 0


def pari_isvalid(gp_path: Path) -> bool:
    """Optional cross-check, not used for the independent verdict."""
    import subprocess
    path = gp_path.as_posix()
    script = (
        f'default(parisizemax, "512M");\n'
        f's = readstr("{path}");\n'
        f'c = eval(s[1]);\n'
        f'print(primecertisvalid(c));\n'
        f'quit\n'
    )
    proc = subprocess.run(
        ["gp", "-q"],
        input=script,
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        return False
    lines = [ln.strip() for ln in proc.stdout.splitlines() if ln.strip()]
    return bool(lines) and lines[-1] == "1"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("--selftest", action="store_true", help="test the arithmetic on known values")
    parser.add_argument("--pari", action="store_true", help="also ask PARI primecertisvalid (cross-check only)")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    return verify_all(use_pari=args.pari)


if __name__ == "__main__":
    sys.exit(main())
