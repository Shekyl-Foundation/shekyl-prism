# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Seed constructions that feed H2-style selection of a norm-T ideal.

Research model only. No constant-time or security claim.

The four ideas Rick asked to compare, plus the H(pk)||T baseline that the
4-D experiment showed is fully recomputable, plus a combination:

  B0   baseline: seed = H(pk || q). Public and recomputable.
  Hpk  public fold: seed = H(pk || msg). Shows what a public input does.
  Hsk  EdDSA-style: seed = H(sk || msg || q). Deterministic from the secret.
  COM  commitment: r = H(sk || msg), c = H(r) goes in the signature,
       q = FS(pk, c, msg), seed = r. Verifier recomputes q from c; not r.
  EPH  ephemeral sub-key: (esk, epk) = H(sk || msg), epk goes in the
       signature, seed = H(epk || msg), q = FS(pk, epk, msg). After the
       signature is public, so is the seed.
  REV  reverse: sample a unimodular map U from H(sk || msg) first, commit
       to d = H(U), set q = FS(pk, d, msg), then select the norm-q ideal
       with seed = H(sk || msg || q). The target is an output of the map.
  CMB  combination: REV's q (so the challenge is bound to a secret-derived
       commitment) and Hsk's seed, which is the same seed REV already uses
       for the ideal. Recorded so the report can say where the pieces go.

H2 itself, and the quaternion lattice, are the 4-D harness
(research/latticeprism-4d/lp4d.py).
"""
import hashlib
import sys
from pathlib import Path

import gmpy2

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'latticeprism-4d'))
from lp4d import (  # noqa: E402
    SETS, prime_of, xof_ints, alpha_for, ideal_basis, gram_norm, lll_gram,
    apply, nrd, unimodular_from_xof, matmul, det4,
)
from run_4d import PK_BYTES  # noqa: E402

DOMAIN = b'LatticePrism/v1/seeding/'


def shake(tag: bytes, *parts: bytes, n=32) -> bytes:
    h = hashlib.shake_256(DOMAIN + tag)
    for p in parts:
        h.update(len(p).to_bytes(4, 'little'))
        h.update(p)
    return h.digest(n)


def keypair(sk: bytes, set_name: str) -> bytes:
    """Simulated public key of the set's length, a deterministic hash of sk."""
    return shake(b'pk', sk, set_name.encode(), n=PK_BYTES[set_name])


def fs_challenge(e: int, *parts: bytes, tag=b'FS'):
    """Odd s of e-4 free bits, q = 2^(2e-5) - s^2, retry until q is prime."""
    ctr = 0
    while True:
        raw = shake(tag, *parts, ctr.to_bytes(4, 'little'), n=(e // 8) + 16)
        s = ((int.from_bytes(raw, 'little') & ((1 << (e - 4)) - 1)) << 1) | 1
        q = (1 << (2 * e - 5)) - s * s
        ctr += 1
        if gmpy2.is_prime(q):
            return s, q, ctr


def h2_select(T: int, p: int, seed: bytes):
    """Rejection-sample a valid (x, y) for the H2 ideal of norm T. No LLL."""
    ctr = 0
    bits = T.bit_length() + 64
    while True:
        y, sgn = xof_ints(seed, T, b'H2' + ctr.to_bytes(4, 'little'), 2, bits)
        y %= T
        x = alpha_for(T, p, y)
        ctr += 1
        if x is not None:
            if sgn & 1:
                x = T - x
            return x, y, ctr


def h2_reduce(T: int, p: int, seed: bytes, xy=None):
    if xy is None:
        x, y, tries = h2_select(T, p, seed)
    else:
        x, y, tries = xy
    B = ideal_basis(T, p, (x, y, 1, 0))
    H, tr = lll_gram(gram_norm(B, p))
    beta = apply(H, B)[0]
    ok = nrd(beta, p) % T == 0 and nrd(beta, p) > 0
    return dict(iterations=tr['iterations'], swaps=tr['swaps'],
                sizered=tr['sizered'], tries=tries, ok=int(ok),
                ratio_bits=(nrd(beta, p) // T).bit_length() if ok else -1)


def encode_U(U):
    out = bytearray()
    for row in U:
        for v in row:
            b = int(v).to_bytes((int(v).bit_length() + 16) // 8 or 1, 'little', signed=True)
            out += len(b).to_bytes(2, 'little') + b
    return bytes(out)


def construction(name, sk, pk, msg, e, p, set_name):
    """Return a dict describing one signing attempt. No LLL here."""
    pub = {'name': name, 'pk': pk, 'msg': msg}
    if name == 'B0':
        s, q, qtries = fs_challenge(e, pk, msg, tag=b'FS-public')
        seed = shake(b'B0', pk, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
        sig = {}  # q is publicly computable from (pk, msg)
        q_mode = 'public-FS(pk,msg)'
    elif name == 'Hpk':
        s, q, qtries = fs_challenge(e, pk, msg, tag=b'FS-public')
        seed = shake(b'Hpk', pk, msg)
        sig = {}
        q_mode = 'public-FS(pk,msg)'
    elif name == 'Hsk':
        s, q, qtries = fs_challenge(e, pk, msg, tag=b'FS-public')
        seed = shake(b'Hsk', sk, msg, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
        sig = {}
        q_mode = 'public-FS(pk,msg)'
    elif name == 'COM':
        r = shake(b'COM-r', sk, msg)
        c = shake(b'COM-c', r)
        s, q, qtries = fs_challenge(e, pk, c, msg, tag=b'FS-commit')
        seed = r
        sig = {'c': c}
        q_mode = 'FS(pk,c,msg)'
    elif name == 'EPH':
        esk = shake(b'EPH-esk', sk, msg, n=32)
        epk = shake(b'EPH-epk', esk, n=32)
        s, q, qtries = fs_challenge(e, pk, epk, msg, tag=b'FS-eph')
        seed = shake(b'EPH-seed', epk, msg)
        sig = {'epk': epk}
        q_mode = 'FS(pk,epk,msg)'
    elif name in ('REV', 'CMB'):
        # Map first: U depends only on (sk, msg), not on q.
        map_seed = shake(b'REV-map', sk, msg)
        U = unimodular_from_xof(map_seed, 0, bits=128)
        d = shake(b'REV-digest', encode_U(U))
        s, q, qtries = fs_challenge(e, pk, d, msg, tag=b'FS-rev')
        seed = shake(b'Hsk', sk, msg, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
        sig = {'d': d}
        q_mode = 'FS(pk,d,msg)'
        pub['U'] = U
        pub['map_seed'] = map_seed
        pub['d'] = d
    else:
        raise ValueError(name)
    pub.update(seed=seed, s=s, q=q, qtries=qtries, sig=sig, q_mode=q_mode)
    return pub


NAMES = ('B0', 'Hpk', 'Hsk', 'COM', 'EPH', 'REV')  # CMB == REV for the q-ideal


def attacker_seeds(name, pk, msg, sig, e):
    """Seeds and q an attacker can derive from public data.

    Returns (before, after): each is a seed or None. `before` uses only
    (pk, msg). `after` also uses signature fields (c, epk, d).
    """
    def q_public():
        return fs_challenge(e, pk, msg, tag=b'FS-public')[1]

    before = after = None
    if name == 'B0':
        q = q_public()
        before = after = shake(b'B0', pk, q.to_bytes((q.bit_length() + 7) // 8, 'little'))
    elif name == 'Hpk':
        before = after = shake(b'Hpk', pk, msg)
    elif name == 'Hsk':
        before = after = None  # needs sk
    elif name == 'COM':
        before = None
        after = None  # c = H(r) is one-way; q is recoverable, r is not
    elif name == 'EPH':
        before = None
        if sig.get('epk'):
            after = shake(b'EPH-seed', sig['epk'], msg)
    elif name in ('REV', 'CMB'):
        before = after = None  # d = H(U) is one-way; U and seed need sk
    return before, after


def attacker_q(name, pk, msg, sig, e):
    """The challenge q the verifier (and an attacker) recomputes."""
    if name in ('B0', 'Hpk', 'Hsk'):
        return fs_challenge(e, pk, msg, tag=b'FS-public')[1]
    if name == 'COM':
        return fs_challenge(e, pk, sig['c'], msg, tag=b'FS-commit')[1]
    if name == 'EPH':
        return fs_challenge(e, pk, sig['epk'], msg, tag=b'FS-eph')[1]
    if name in ('REV', 'CMB'):
        return fs_challenge(e, pk, sig['d'], msg, tag=b'FS-rev')[1]
    raise ValueError(name)
