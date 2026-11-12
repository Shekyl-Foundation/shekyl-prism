# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Accept-path model for PRISM §5 (Step 2 brief §4).
# Uses Pierrick-Dartois/Theta_dim4 @ 4770b24 unmodified.

import os
import sys
import json
import subprocess
from pathlib import Path

# Sage's run-file copies this script to a temp path. The repo is the
# working directory, or PRISM_ROOT when that is set. There is no fallback
# path: a missing root is a failed run.
def find_root():
    candidates = []
    if os.environ.get("PRISM_ROOT"):
        candidates.append(Path(os.environ["PRISM_ROOT"]))
    candidates.append(Path.cwd())
    here = Path(__file__).resolve()
    candidates.append(here.parents[2] if len(here.parents) > 2 else here)
    for candidate in candidates:
        if (candidate / "external" / "Theta_dim4").is_dir():
            return candidate
    raise SystemExit(
        "shekyl-prism root not found (set PRISM_ROOT, or run from the repo)"
    )


ROOT = find_root()
sys.path.insert(0, str(ROOT / "external" / "Theta_dim4" / "Theta_dim4_sage"))

from sage.all import (
    EllipticCurve,
    GF,
    Integers,
    Matrix,
    QQ,
    ZZ,
    ceil,
    identity_matrix,
    inverse_mod,
    matrix,
    vector,
)
from pkg.basis_change.base_change_dim2 import (
    base_change_theta_dim2,
    complete_symplectic_matrix_dim2,
    is_symplectic_matrix_dim2,
)
from pkg.basis_change.canonical_basis_dim1 import make_canonical
from pkg.isogenies_dim2.isogeny_chain_dim2 import IsogenyChainDim2
from pkg.theta_structures.Theta_dim1 import ThetaStructureDim1
from pkg.theta_structures.Theta_dim2 import ProductThetaStructureDim2
from pkg.theta_structures.Tuple_point import TuplePoint
from pkg.utilities.discrete_log import weil_pairing_pari

LEVELS = {
    "NIST-I": {"e": 248, "odd": 5, "lambda": 128, "fp_len": 32},
    "NIST-III": {"e": 376, "odd": 65, "lambda": 192, "fp_len": 48},
    "NIST-V": {"e": 500, "odd": 27, "lambda": 256, "fp_len": 64},
}

VECTORS = [
    ROOT / "prism-verify" / "testdata" / "s5_lvl1.txt",
    ROOT / "prism-verify" / "testdata" / "s5_lvl3.txt",
    ROOT / "prism-verify" / "testdata" / "s5_lvl5.txt",
    ROOT / "prism-verify" / "testdata" / "s5_lvl1b.txt",
    ROOT / "prism-verify" / "testdata" / "s5_lvl3b.txt",
    ROOT / "prism-verify" / "testdata" / "s5_lvl5b.txt",
]

OUT = ROOT / "prism-isogeny" / "testdata" / "s5_model"
OUT.mkdir(parents=True, exist_ok=True)


def claim(kind, name, detail=""):
    tag = {"Ran": "RAN", "Derived": "DERIVED", "Open": "OPEN"}[kind]
    line = f"[{tag}] {name}"
    if detail:
        line += f": {detail}"
    print(line, flush=True)


def fail(name, detail):
    if getattr(fail, "soft", False):
        return
    print(f"[REFUTED] {name}: {detail}", flush=True)
    sys.exit(1)


def load_vector(path):
    rows = {}
    for line in path.read_text().splitlines():
        if not line or " " not in line:
            continue
        key, value = line.split(" ", 1)
        rows[key] = value
    return rows


def fp2_from_hex(hex_str, Fp2, fp_len):
    raw = bytes.fromhex(hex_str)
    if len(raw) != 2 * fp_len:
        raise ValueError(f"expected {2*fp_len} bytes, got {len(raw)}")
    re = int.from_bytes(raw[:fp_len], "little")
    im = int.from_bytes(raw[fp_len:], "little")
    return Fp2([re, im])


def montgomery(A):
    return EllipticCurve(A.parent(), [0, A, 0, 1, 0])


def on_curve(E, x, y):
    return E.is_on_curve(x, y)


def point(E, x, y):
    return E(x, y)


def rhs_square(A, x):
    return (x**3 + A * x**2 + x).is_square()


def basis_index(A, e, odd, lam):
    """The verifier's index search.

    `1 + b^2` is tested in `F_p`. Testing it as an integer square accepts
    an index whose abscissa lies on the quadratic twist.
    """
    bound = 2 * lam
    prime = A.parent().characteristic()
    base = GF(prime)
    square = A.is_square()
    Fp2 = A.parent()
    i = Fp2.gen()
    assert i**2 == -1
    if square:
        for b in range(1, bound + 1):
            if base(1 + b * b).is_square():
                continue
            z = 1 + i * ZZ(b)
            # A^2 (z-1) - z^2 non-square
            if not ((A * A) * (z - 1) - z * z).is_square():
                return ZZ(b), True
    else:
        x = A
        for n in range(1, bound + 1):
            if rhs_square(A, x):
                return ZZ(n), False
            x = x + A
    raise ValueError("basis index search failed")


def abscissa(A, index, square):
    Fp2 = A.parent()
    i = Fp2.gen()
    if square:
        z = 1 + i * ZZ(index)
        return -A / z
    return A * ZZ(index)


def clear_odd(E, P, odd):
    return odd * P


def even_generators(E, A, index, square, odd):
    """Order-2^e generators: clear the odd cofactor of the full preimages."""
    xp = abscissa(A, index, square)
    xq = -xp - A
    # Lift y: take a square root of the RHS
    yp2 = xp**3 + A * xp**2 + xp
    yp = yp2.sqrt()
    yq2 = xq**3 + A * xq**2 + xq
    yq = yq2.sqrt()
    Pfull = E(xp, yp)
    Qfull = E(xq, yq)
    # Match exported sign of y when available is done by the caller.
    P = clear_odd(E, Pfull, odd)
    Q = clear_odd(E, Qfull, odd)
    return P, Q, Pfull, Qfull


def projective_equal(coords_a, coords_b):
    a = list(coords_a)
    b = list(coords_b)
    for i, ai in enumerate(a):
        if ai != 0:
            if b[i] == 0:
                return False
            scale = b[i] / ai
            return all(bj == scale * aj for aj, bj in zip(a, b))
    return all(bj == 0 for bj in b)


def phi_matrix(s, q, modulus):
    """Matrix of Φ in (b1,b2,b3,b4) over Z/modulus (column action)."""
    Zn = Integers(modulus)
    s, q = Zn(s), Zn(q)
    return matrix(
        Zn,
        [
            [s, -q, 0, 0],
            [1, s, 0, 0],
            [0, 0, s, -1],
            [0, 0, q, s],
        ],
    )


def phi_hat_matrix(s, q, modulus):
    Zn = Integers(modulus)
    s, q = Zn(s), Zn(q)
    # Adjugate-like dual for Φ̂ with Φ̂Φ = (s^2+q) I; from the brief:
    # Φ̂(b1)=s b1 - b2?  From Φ̂Φ = N I and the printed map.
    # Φ(X,Y)=([s]X - σ̂(Y), σ(X)+[s]Y). Dual swaps and conjugates.
    # From the matrix of Φ, Φ̂ = N Φ^{-1} = [[s,q,0,0],[-1,s,0,0],[0,0,s,1],[0,0,-q,s]]
    # Check: brief Φ̂(P_sig)= -[s]P_vk on the relevant torsion.
    return matrix(
        Zn,
        [
            [s, q, 0, 0],
            [-1, s, 0, 0],
            [0, 0, s, 1],
            [0, 0, -q, s],
        ],
    )


def pairing_power(E, P, Q, order, exponent):
    return weil_pairing_pari(P, Q, order) ** exponent


def closed_form_matrices(s, modulus):
    """Columns in `(b1, b2, b3, b4)`, with `t = s^{-1}` modulo `2^e`.

    `M1 = [(0,0,−t,0), (0,t²,0,0) | (s,−1,0,0), (0,0,s,s²)]`
    `M2 = [(−1,0,0,0), (0,0,0,−s) | (0,0,−1,s), (1,t,0,0)]`

    The right-hand pair of columns is the kernel half and the left-hand
    pair is the complement. `q ≡ −s² (mod 2^e)`, so the entries use `s`
    only.
    """
    Zn = Integers(modulus)
    s = Zn(s)
    t = s**-1
    M1 = matrix(
        Zn,
        [
            [0, 0, s, 0],
            [0, t**2, -1, 0],
            [-t, 0, 0, s],
            [0, 0, 0, s**2],
        ],
    )
    M2 = matrix(
        Zn,
        [
            [-1, 0, 0, 1],
            [0, 0, 0, t],
            [0, 0, -1, 0],
            [0, -s, s, 0],
        ],
    )
    return M1, M2


def assert_closed_form(s, q, modulus, path):
    """Symplectic, and the two kernel halves are the images under Φ and −Φ̂."""
    Mphi = phi_matrix(s, q, modulus)
    Mhat = phi_hat_matrix(s, q, modulus)
    M1, M2 = closed_form_matrices(s, modulus)
    if not is_symplectic_matrix_dim2(M1):
        fail(f"{path.name} closed M1", "not symplectic")
    if not is_symplectic_matrix_dim2(M2):
        fail(f"{path.name} closed M2", "not symplectic")
    if M2.matrix_from_columns([2, 3]) != Mphi * M1.matrix_from_columns([0, 1]):
        fail(f"{path.name} closed M2 kernel", "not Φ of the M1 complement")
    if M1.matrix_from_columns([2, 3]) != -(Mhat * M2.matrix_from_columns([0, 1])):
        fail(f"{path.name} closed M1 kernel", "not −Φ̂ of the M2 complement")
    return M1, M2


def library_symplectic(path, order, Mphi, Mhat):
    """Dartois's completion: M2 completed, M1 glued and not re-completed."""
    Zn = Integers(order)
    s_mod = Mphi[0, 0]
    C1 = matrix(Zn, [[s_mod, 0], [-1, 0]])
    D1 = matrix(Zn, [[0, s_mod], [0, Mphi[0, 1]]])
    M1_0 = complete_symplectic_matrix_dim2(C1, D1, order)
    if not is_symplectic_matrix_dim2(M1_0):
        fail(f"{path.name} M1_0", "not symplectic")
    complement = M1_0.matrix_from_columns([0, 1])
    block_right2 = Mphi * complement
    C2 = block_right2.matrix_from_rows([0, 1])
    D2 = block_right2.matrix_from_rows([2, 3])
    if C2.transpose() * D2 != D2.transpose() * C2:
        fail(f"{path.name} M2 kernel half", "not isotropic")
    M2 = complete_symplectic_matrix_dim2(C2, D2, order)
    if not is_symplectic_matrix_dim2(M2):
        fail(f"{path.name} M2", "not symplectic")
    block_right1 = Mhat * M2.matrix_from_columns([0, 1])
    C1b = block_right1.matrix_from_rows([0, 1])
    D1b = block_right1.matrix_from_rows([2, 3])
    if C1b.transpose() * D1b != D1b.transpose() * C1b:
        fail(f"{path.name} M1 kernel half", "not isotropic")
    A_blk = M1_0.matrix_from_rows_and_columns([0, 1], [0, 1])
    B_blk = M1_0.matrix_from_rows_and_columns([2, 3], [0, 1])
    if B_blk.transpose() * A_blk != A_blk.transpose() * B_blk:
        fail(f"{path.name} M1_0 complement", "not isotropic")
    M1 = complement.augment(-block_right1)
    if not is_symplectic_matrix_dim2(M1):
        fail(f"{path.name} M1", "glued [complement | −Φ̂·compl(M2)] not symplectic")
    claim("Derived", f"{path.name} symplectic matrices", "confirmed")
    return M1, M2


def evaluate_chains(B_K1, B_K2, Theta12, N1, N2, e_prime, P1, P2, Psig, Qsig, s, Esig):
    """Codomain and step 8. Returns (verdict, detail) with verdict in
    Kernel / Codomain / Image / Ok."""
    try:
        chain1 = IsogenyChainDim2(B_K1, Theta12, N1, e_prime)
        chain2 = IsogenyChainDim2(B_K2, Theta12, N2, e_prime - 1)
    except Exception as err:
        return "Kernel", {"error": str(err)}
    C1z = chain1._codomain.zero().coords()
    C2hz = chain2._codomain.hadamard().zero().coords()
    nulls1 = [[str(c) for c in phi._codomain.zero().coords()] for phi in chain1._phis]
    nulls2 = [[str(c) for c in phi._codomain.zero().coords()] for phi in chain2._phis]
    detail = {"nulls1": nulls1, "nulls2": nulls2, "step8": {}}
    if not projective_equal(C1z, C2hz):
        return "Codomain", detail
    dual2 = chain2.dual()

    def Phi(pt):
        return dual2(chain1(pt))

    images = {}
    for name, X, Y in [
        ("P", P1, Psig),
        ("Q", P2, Qsig),
        ("PmQ", P1 - P2, Psig - Qsig),
    ]:
        try:
            img = Phi(TuplePoint(X, Esig(0)))
        except Exception as err:
            detail["error"] = str(err)
            return "Kernel", detail
        target_x = s * X
        target_y = s * Y
        left_ok = img[0] in (target_x, -target_x)
        right_ok = img[1] in (target_y, -target_y)
        images[name] = {
            "left_ok": left_ok,
            "right_ok": right_ok,
            "img0": str(img[0]),
            "img1": str(img[1]),
        }
        if not (left_ok and right_ok):
            detail["step8"] = images
            return "Image", detail
    detail["step8"] = images
    return "Ok", detail


def run_vector(path, run_chains=True, soft=None, completion="closed"):
    """`soft` is a mutation. Algebraic `fail` calls are skipped and the
    chain verdict is returned, so a negative is classified by Codomain,
    Image, or Kernel rather than by an identity the verifier does not check.
    """
    fail.soft = soft is not None
    rows = load_vector(path)
    level = rows["level"]
    params = LEVELS[level]
    e = params["e"]
    e_prime = e - 2
    odd = params["odd"]
    lam = params["lambda"]
    fp_len = params["fp_len"]
    p = odd * (1 << e) - 1
    assert p % 4 == 3
    Fp2 = GF(p**2, name="i", modulus=ZZ["x"]("x^2+1"))
    i = Fp2.gen()
    assert i**2 == -1

    A = fp2_from_hex(rows["A"], Fp2, fp_len)
    Esig_A = fp2_from_hex(rows["E"], Fp2, fp_len)
    s = ZZ(rows["s"])
    q = ZZ(rows["q"])
    assert s**2 + q == 1 << (2 * e_prime - 1)
    assert s % 2 == 1

    Evk = montgomery(A)
    Esig = montgomery(Esig_A)

    Psig = point(Esig, fp2_from_hex(rows["Px"], Fp2, fp_len), fp2_from_hex(rows["Py"], Fp2, fp_len))
    Qsig = point(Esig, fp2_from_hex(rows["Qx"], Fp2, fp_len), fp2_from_hex(rows["Qy"], Fp2, fp_len))
    Pvk = point(Evk, fp2_from_hex(rows["VkPx"], Fp2, fp_len), fp2_from_hex(rows["VkPy"], Fp2, fp_len))
    Qvk = point(Evk, fp2_from_hex(rows["VkQx"], Fp2, fp_len), fp2_from_hex(rows["VkQy"], Fp2, fp_len))

    # Orders. A mutation that drops the order is Kernel, before any chain.
    order = 1 << e

    def exact_order(pt):
        return (order * pt).is_zero() and not ((order // 2) * pt).is_zero()

    if soft is None:
        assert exact_order(Pvk) and exact_order(Qvk)
        assert exact_order(Psig) and exact_order(Qsig)
    elif not exact_order(Psig) or not exact_order(Qsig):
        return "Kernel"

    # Independent basis index (§4.1)
    index, square = basis_index(A, e, odd, lam)
    claim("Ran", f"{path.name} basis index", f"index={index} square={square}")
    xp = abscissa(A, index, square)
    xr = -xp - A
    # The search names a point on the curve. An abscissa on the twist is a
    # failed search, not a vector the export is trusted to repair.
    if not rhs_square(A, xp):
        fail(
            f"{path.name} index abscissa",
            f"not on the curve (index {index}, square={square})",
        )
    Pclr = clear_odd(Evk, Evk.lift_x(xp), odd)
    if Pclr[0] != Pvk[0] and (-Pclr)[0] != Pvk[0]:
        fail(f"{path.name} VkP", "derived cleared P disagrees with export")
    claim("Ran", f"{path.name} VkP matches cleared index abscissa", "")
    # The exporter emits the preimage pair itself: (VkP, VkQ) is, up to one
    # global sign, the C canonical basis (P, Q) whose t*sigma image is the
    # signature. VkDx cross-checks the pair. (The earlier export swapped
    # Q and PmQ, emitting (P, P-Q); the instrumented exporter run recorded
    # in the report established the correspondence.)
    P1 = Pvk
    P2 = Qvk
    if soft is not None:
        if "Psig" in soft:
            Psig = soft["Psig"]
        if "Qsig" in soft:
            Qsig = soft["Qsig"]
        if "Esig" in soft:
            Esig = soft["Esig"]
        if "s" in soft:
            s = soft["s"]
            q = soft["q"]
        if "P1" in soft:
            P1 = soft["P1"]
            P2 = soft["P2"]
            Evk = P1.curve()
        if not exact_order(Psig) or not exact_order(Qsig):
            return "Kernel"
        p2 = (order // 2) * Psig
        q2 = (order // 2) * Qsig
        if p2.is_zero() or q2.is_zero() or p2[0] == q2[0]:
            return "Kernel"
    if soft is None and "VkDx" in rows:
        if fp2_from_hex(rows["VkDx"], Fp2, fp_len) != (P1 - P2)[0]:
            fail(f"{path.name} VkDx", "x(VkP - VkQ) differs from VkDx")
        claim("Ran", f"{path.name} VkDx = x(VkP-VkQ)", "")

    # §3 independent validation: e(R1,R2) = e(P1,P2)^q. A global sign on
    # the exported pair is [-1] composed into sigma and cancels here; any
    # mismatch is an exporter regression, not something to repair.
    R1 = s * Psig
    R2 = s * Qsig
    left = weil_pairing_pari(R1, R2, order)
    right = weil_pairing_pari(P1, P2, order) ** q
    if left != right:
        # After order and span, before any chain. A mismatch is Pairing
        # whether or not the chains would have finished.
        if soft is not None:
            return "Pairing"
        fail(f"{path.name} Pairing", "e(R1,R2) != e(P1,P2)^q")
    claim("Ran", f"{path.name} Weil e(R1,R2)=e(P1,P2)^q", "")
    lamb = inverse_mod(q, order)
    # Symplectic product basis (§2a): e(P1,P2)=e(R1,[λ]R2)
    if weil_pairing_pari(P1, P2, order) != weil_pairing_pari(R1, lamb * R2, order):
        fail(f"{path.name} symplectic", "pairing after [s] disagrees")
    claim("Derived", f"{path.name} product basis symplectic after [s]", "confirmed")

    # Φ matrix (§4.2)
    Ndeg = 1 << (2 * e_prime - 1)
    Mphi = phi_matrix(s, q, order)
    Mhat = phi_hat_matrix(s, q, order)
    prod = Mhat * Mphi
    expected = identity_matrix(Integers(order), 4) * Integers(order)(Ndeg)
    if prod != expected:
        fail(f"{path.name} Φ̂Φ", f"got {prod}, want {Ndeg}*I")
    claim("Derived", f"{path.name} Φ̂Φ = 2^{{2e'-1}} I", "confirmed")

    # Kernel lifts (§4.3)
    Zn = Integers(order)
    # Generators of K″1 as columns of [C;D]
    C1 = matrix(Zn, [[s, 0], [-1, 0]])
    D1 = matrix(Zn, [[0, s], [0, -q]])
    # Isotropy: C^T D = D^T C
    if C1.transpose() * D1 != D1.transpose() * C1:
        fail(f"{path.name} K″1 isotropic", "C^T D != D^T C")
    # Pairing check on the geometric generators
    g1 = TuplePoint(s * P1, -R1)
    g2 = TuplePoint(s * P2, -R2)
    # Order 2^e
    if not (g1 * order).is_zero() or (g1 * (order // 2)).is_zero():
        fail(f"{path.name} K″1 g1 order", "")
    if not (g2 * order).is_zero() or (g2 * (order // 2)).is_zero():
        fail(f"{path.name} K″1 g2 order", "")
    # Weil pairing on product: e((X1,Y1),(X2,Y2)) = e(X1,X2)*e(Y1,Y2)
    e_prod = weil_pairing_pari(g1[0], g2[0], order) * weil_pairing_pari(g1[1], g2[1], order)
    if e_prod != 1:
        fail(f"{path.name} K″1 geometric isotropic", f"pairing={e_prod}")
    claim("Derived", f"{path.name} K″1 isotropic", "confirmed")

    h1 = TuplePoint(s * P1, R1)
    h2 = TuplePoint(s * P2, R2)
    k1 = 2 * h1
    k2 = 2 * h2
    e_prod2 = weil_pairing_pari(k1[0], k2[0], order // 2) * weil_pairing_pari(
        k1[1], k2[1], order // 2
    )
    if e_prod2 != 1:
        fail(f"{path.name} K″2 isotropic", f"pairing={e_prod2}")
    claim("Derived", f"{path.name} K″2 isotropic", "confirmed")

    # Symplectic matrices (§4.4). `library` is Dartois's completion.
    # `closed` is the representative with t = s^{-1}: the kernel half of
    # M2 is Φ of M1's complement, and the kernel half of M1 is −Φ̂ of
    # M2's complement.
    if completion == "closed":
        M1, M2 = assert_closed_form(s, q, order, path)
        claim("Derived", f"{path.name} closed-form symplectic matrices", "confirmed")
    elif completion == "library":
        M1, M2 = library_symplectic(path, order, Mphi, Mhat)
    else:
        fail(f"{path.name} completion", f"unknown completion {completion}")

    # Product theta structure (§4 / Spec)
    # Order-4 from doubling e-2 times
    T_P = (1 << (e - 2)) * P1
    T_Q = (1 << (e - 2)) * P2
    U_R = (1 << (e - 2)) * R1
    U_S = (1 << (e - 2)) * (lamb * R2)
    _, _, T1, T2, MT = make_canonical(T_P, T_Q, 4, preserve_pairing=True)
    _, _, U1, U2, MU = make_canonical(U_R, U_S, 4, preserve_pairing=True)
    Z4 = Integers(4)
    M0 = matrix(
        Z4,
        [
            [MT[0, 0], 0, MT[1, 0], 0],
            [0, MU[0, 0], 0, MU[1, 0]],
            [MT[0, 1], 0, MT[1, 1], 0],
            [0, MU[0, 1], 0, MU[1, 1]],
        ],
    )
    Theta1 = ThetaStructureDim1(Evk, T1, T2)
    Theta2 = ThetaStructureDim1(Esig, U1, U2)
    Theta12 = ProductThetaStructureDim2(Theta1, Theta2)
    e4 = Fp2(weil_pairing_pari(T1, T2, 4))

    # Reduce M1, M2 mod 4 for theta base change
    M1_4 = matrix(Z4, [[ZZ(M1[i, j]) % 4 for j in range(4)] for i in range(4)])
    M2_4 = matrix(Z4, [[ZZ(M2[i, j]) % 4 for j in range(4)] for i in range(4)])
    N1 = base_change_theta_dim2(M0 * M1_4, e4)
    N2 = base_change_theta_dim2(M0 * M2_4, e4)

    # Chain kernels (§4.5): the kernel-half columns of M1, M2 evaluated on
    # the basis (b1..b4), as kernel_basis does in the dim-4 code. N_i is
    # adapted to exactly this basis; the hand-written K″ generators span the
    # right group mod 4 but are a different basis (and for M1 a different
    # isotropic lift), which breaks the codomain comparison.
    def matrix_kernel_basis(M, ei):
        m = 1 << (e - ei - 2)
        modulus = 1 << (ei + 2)
        mP1, mP2 = m * P1, m * P2
        mR1, mlR2 = m * R1, m * (lamb * R2)
        pts = []
        for j in (2, 3):
            c = [ZZ(M[k, j]) % modulus for k in range(4)]
            pts.append(TuplePoint(c[0] * mP1 + c[2] * mP2, c[1] * mR1 + c[3] * mlR2))
        return pts

    B_K1 = matrix_kernel_basis(M1, e_prime)
    B_K2 = matrix_kernel_basis(M2, e_prime - 1)
    for kname, pts, ei in (("K1", B_K1, e_prime), ("K2", B_K2, e_prime - 1)):
        o = 1 << (ei + 2)
        for t, pt in enumerate(pts):
            if not (pt * o).is_zero() or (pt * (o // 2)).is_zero():
                fail(f"{path.name} chain kernel {kname}[{t}]", f"order is not 2^{ei+2}")

    stem = path.stem
    kat = {
        "level": level,
        "e": int(e),
        "e_prime": int(e_prime),
        "s": str(s),
        "q": str(q),
        "index": int(index),
        "square": bool(square),
        "Mphi": [[str(Mphi[i, j]) for j in range(4)] for i in range(4)],
        "M1": [[str(M1[i, j]) for j in range(4)] for i in range(4)],
        "M2": [[str(M2[i, j]) for j in range(4)] for i in range(4)],
        "M1_4": [[int(M1_4[i, j]) for j in range(4)] for i in range(4)],
        "M2_4": [[int(M2_4[i, j]) for j in range(4)] for i in range(4)],
        "e4": str(e4),
        "Mbc1": [[int(ZZ(entry)) for entry in row] for row in (M0 * M1_4)],
        "Mbc2": [[int(ZZ(entry)) for entry in row] for row in (M0 * M2_4)],
        "N1": [[str(N1[i, j]) for j in range(4)] for i in range(4)],
        "N2": [[str(N2[i, j]) for j in range(4)] for i in range(4)],
        "BK1": [
            [str(pt[0][0]), str(pt[0][1]), str(pt[1][0]), str(pt[1][1])] for pt in B_K1
        ],
        "BK2": [
            [str(pt[0][0]), str(pt[0][1]), str(pt[1][0]), str(pt[1][1])] for pt in B_K2
        ],
    }
    comparing = os.environ.get("S5_COMPARE") == "1"
    # Known-answer files are the six original vectors. Accept-set files are
    # verdicts only; writing a matrix file for each of them is not the record.
    original = {item.name for item in (
        ROOT / "prism-verify" / "testdata" / f"s5_lvl{n}.txt"
        for n in ("1", "1b", "3", "3b", "5", "5b")
    )}
    if soft is None and not comparing and path.name in original:
        (OUT / f"{stem}_matrices.json").write_text(json.dumps(kat, indent=2) + "\n")

    algebra_only = os.environ.get("S5_ALGEBRA_ONLY") == "1" and not comparing
    if not run_chains or algebra_only:
        return "algebra"

    try:
        verdict, images = evaluate_chains(
            B_K1, B_K2, Theta12, N1, N2, e_prime, P1, P2, Psig, Qsig, s, Esig
        )
    except Exception:
        return "Kernel"
    if soft is not None or comparing:
        return verdict
    if verdict != "Ok":
        fail.soft = False
        fail(f"{path.name} {verdict}", "honest vector")
    fail.soft = False
    if path.name in original:
        nulls1 = images["nulls1"]
        nulls2 = images["nulls2"]
        (OUT / f"{stem}_nulls.json").write_text(
            json.dumps({"chain1": nulls1, "chain2": nulls2}, indent=2) + "\n"
        )
        (OUT / f"{stem}_step8.json").write_text(json.dumps(images["step8"], indent=2) + "\n")
    claim("Derived", f"{path.name} Codomain C1.zero()==C2.hadamard().zero()", "confirmed")
    claim("Derived", f"{path.name} Image step 8", "confirmed")
    claim("Ran", f"{path.name} accept path Ok", "")
    return "Ok"


SHORT = {"NIST-I": "I", "NIST-III": "III", "NIST-V": "V"}
SIBLING = {
    "s5_lvl1.txt": "s5_lvl1b.txt",
    "s5_lvl1b.txt": "s5_lvl1.txt",
    "s5_lvl3.txt": "s5_lvl3b.txt",
    "s5_lvl3b.txt": "s5_lvl3.txt",
    "s5_lvl5.txt": "s5_lvl5b.txt",
    "s5_lvl5b.txt": "s5_lvl5.txt",
}


def parts(z):
    coeffs = list(z.polynomial())
    re = ZZ(coeffs[0]) if coeffs else ZZ(0)
    im = ZZ(coeffs[1]) if len(coeffs) > 1 else ZZ(0)
    return re, im


def ext_less(left, right):
    left_re, left_im = parts(left)
    right_re, right_im = parts(right)
    if left_re != right_re:
        return left_re < right_re
    return left_im < right_im


def canonical_sign(y):
    return y != 0 and ext_less(y, -y)


def montgomery_coefficients(A):
    """The Montgomery coefficients of A's isomorphism class that the
    exporter enumerates: ±A, and ± the two moves of a rational 2-torsion
    point to (0, 0)."""
    found = [A, -A]
    disc = A**2 - 4
    if not disc.is_square():
        return found
    root = disc.sqrt()
    for alpha in ((-A + root) / 2, (-A - root) / 2):
        lam2 = alpha**2 - 1
        if lam2 == 0 or not lam2.is_square():
            continue
        lam = lam2.sqrt()
        moved = (3 * alpha + A) / lam
        found.append(moved)
        found.append(-moved)
    return found


def is_least_model(A):
    best = A
    for coefficient in montgomery_coefficients(A):
        if ext_less(coefficient, best):
            best = coefficient
    return best == A


def rust_binary(name):
    return Path(os.environ.get("PRISM_RUST_BIN_DIR", ROOT / "target" / "release")) / name


def challenge_scalar(level, coefficient_hex, message, salt):
    binary = rust_binary("s5_challenge")
    proc = subprocess.run(
        [str(binary), SHORT[level], coefficient_hex, message, salt],
        check=True,
        capture_output=True,
        text=True,
    )
    scalar_hex, verdict = proc.stdout.split()
    scalar = ZZ(int.from_bytes(bytes.fromhex(scalar_hex), "little"))
    return scalar, verdict


def transport(A, P, Q, alpha):
    lam2 = alpha**2 - 1
    if lam2 == 0 or not lam2.is_square():
        return None
    for lam in (lam2.sqrt(), -lam2.sqrt()):
        if lam == 0 or not lam.is_square():
            continue
        scale = lam * lam.sqrt()
        if scale == 0:
            continue
        coefficient = (3 * alpha + A) / lam
        curve = montgomery(coefficient)

        def image(point, lam=lam, scale=scale, alpha=alpha, curve=curve):
            return curve((point[0] - alpha) / lam, point[1] / scale)

        try:
            return curve, coefficient, image(P), image(Q)
        except Exception:
            continue
    return None


def negate_model(A, P, Q):
    i = A.parent().gen()
    curve = montgomery(-A)
    return curve, -A, curve(-P[0], i * P[1]), curve(-Q[0], i * Q[1])


def other_models(A, P, Q):
    """The five Montgomery models other than (A, P, Q), points transported."""
    models = [negate_model(A, P, Q)]
    disc = A**2 - 4
    if not disc.is_square():
        return models
    root = disc.sqrt()
    for alpha in ((-A + root) / 2, (-A - root) / 2):
        moved = transport(A, P, Q, alpha)
        if moved is None:
            continue
        models.append(moved)
        curve, coefficient, left, right = moved
        models.append(negate_model(coefficient, left, right))
    # Drop anything whose coefficient is A's, and keep five when the class has six.
    others = [model for model in models if model[1] != A]
    return others


def normalize_joint(left, right):
    """Negate both points or neither, decided by `left`."""
    if left[1] == 0:
        return left, right
    if canonical_sign(left[1]):
        return left, right
    return -left, -right


def item7(path, curve, left_point, right_point, completion="library"):
    """Translates of the signature pair by every nonzero point of `E[4]`."""
    seven = []
    for torsion in row7_translates(curve):
        order = 2 if (2 * torsion).is_zero() else 4
        for label, raw_left, raw_right in (
            ("P", left_point + torsion, right_point),
            ("Q", left_point, right_point + torsion),
            ("both", left_point + torsion, right_point + torsion),
        ):
            left, right = normalize_joint(raw_left, raw_right)
            tag = f"ord{order}:{label}"
            if left[1] == 0:
                seven.append(f"{tag}:Sign")
                continue
            verdict = chain_verdict(path, completion, Psig=left, Qsig=right)
            seven.append(f"{tag}:{verdict}")
            if verdict == "Ok" and os.environ.get("S5_COMPARE") != "1":
                fail(f"{path.name} item 7", f"{tag} accepted")
    return seven


def row7_translates(curve):
    """Nonzero points of `E[4]`, including the three points of `E[2]`.

    An order-2 translate is a different input from an order-4 translate:
    `[2](P+T) = [2]P` when `T` has order 2, and `[2](P+T) = [2]P + [2]T`
    when `T` has order 4. Row 7 records both.
    """
    found = []
    for point in curve(0).division_points(4):
        if point.is_zero():
            continue
        found.append(point)
    return found


def chain_verdict(path, completion="library", **soft):
    try:
        try:
            verdict = run_vector(path, run_chains=True, soft=soft, completion=completion)
        except Exception as err:
            if os.environ.get("S5_CHAIN_EXC") == "1":
                print(f"  chain exception: {type(err).__name__}: {err}", flush=True)
            return "Kernel"
        if verdict in ("Ok", "Kernel", "Pairing", "Codomain", "Image"):
            return verdict
        return "Kernel"
    finally:
        fail.soft = False


def candidate_salt(level, counter):
    width = LEVELS[level]["fp_len"]
    return (int(counter).to_bytes(8, "little") + bytes(width - 8)).hex()


def first_prime_salt(level, key_hex, message, start=0, limit=65536):
    for counter in range(start, min(start + limit, 65536)):
        salt = candidate_salt(level, counter)
        scalar, prime = challenge_scalar(level, key_hex, message, salt)
        if prime == "prime":
            return salt, scalar
    return None, None


def run_negatives(completion="library"):
    """§6 on both keys at each level. Chain verdicts use the accept path."""
    primary = list(VECTORS)
    report = {}
    for path in primary:
        rows = load_vector(path)
        level = rows["level"]
        params = LEVELS[level]
        e = params["e"]
        e_prime = e - 2
        fp_len = params["fp_len"]
        p = params["odd"] * (1 << e) - 1
        Fp2 = GF(p**2, name="i", modulus=ZZ["x"]("x^2+1"))
        A_sig = fp2_from_hex(rows["E"], Fp2, fp_len)
        Esig = montgomery(A_sig)
        P = point(Esig, fp2_from_hex(rows["Px"], Fp2, fp_len), fp2_from_hex(rows["Py"], Fp2, fp_len))
        Q = point(Esig, fp2_from_hex(rows["Qx"], Fp2, fp_len), fp2_from_hex(rows["Qy"], Fp2, fp_len))
        message = rows["message"]
        salt = rows["salt"]
        key = rows["A"]
        name = path.name
        row = {}
        print(f"=== negatives {name} ===", flush=True)

        if not is_least_model(A_sig):
            fail(f"{name} Model self-check", "honest E_sig is not the least model")
        if not canonical_sign(P[1]):
            fail(f"{name} Sign self-check", "honest P fails the sign check")

        # 1. a salt whose degree is composite; no carried search index.
        composite_at = None
        for counter in range(65536):
            candidate = candidate_salt(level, counter)
            _scalar, verdict = challenge_scalar(level, key, message, candidate)
            if verdict == "composite":
                composite_at = candidate
                break
        row["1"] = "Composite" if composite_at is not None else "MISSING"
        print(f"  1 -> {row['1']}", flush=True)

        # 2. a searched salt whose degree is prime, with unchanged response.
        next_prime, next_s = first_prime_salt(level, key, message)
        if next_prime == salt:
            next_prime, next_s = first_prime_salt(level, key, message, start=1)
        if next_prime is None:
            row["2"] = "MISSING"
        else:
            next_q = (1 << (2 * e_prime - 1)) - next_s**2
            row["2"] = chain_verdict(path, completion, s=next_s, q=next_q)
        print(f"  2 -> {row['2']}", flush=True)

        # 3. both signs flipped
        row["3"] = (
            "Sign"
            if not canonical_sign((-P)[1])
            else chain_verdict(path, completion, Psig=-P, Qsig=-Q)
        )
        print(f"  3 -> {row['3']}", flush=True)

        # 4. relative sign
        if not canonical_sign(P[1]):
            row["4"] = "Sign"
        else:
            row["4"] = chain_verdict(path, completion, Qsig=-Q)
        print(f"  4 -> {row['4']}", flush=True)

        # 5. swapped, then normalized jointly. Only row 3 may report Sign.
        swapped_p, swapped_q = normalize_joint(Q, P)
        row["5"] = chain_verdict(path, completion, Psig=swapped_p, Qsig=swapped_q)
        print(f"  5 -> {row['5']}", flush=True)

        # 6. one byte of A_sig flipped. Sage's preparser can swallow
        # `^=` on a bytearray, so the xor is a Python int assignment.
        raw = bytearray(bytes.fromhex(rows["E"]))
        # `^` is exponentiation in Sage, and a bare `1` is a Sage integer.
        raw[0] = int(raw[0]).__xor__(int("1"))
        re = int.from_bytes(raw[:fp_len], "little")
        im = int.from_bytes(raw[fp_len:], "little")
        if re >= p or im >= p:
            row["6"] = "Canonical"
        else:
            flipped = Fp2([re, im])
            if flipped == A_sig:
                fail(f"{name} item 6", "the byte flip did not change A_sig")
            flipped_curve = montgomery(flipped)
            if flipped**2 == 4:
                row["6"] = "Singular"
            elif not is_least_model(flipped):
                row["6"] = "Model"
            elif not (
                flipped_curve.is_on_curve(P[0], P[1])
                and flipped_curve.is_on_curve(Q[0], Q[1])
            ):
                row["6"] = "Kernel"
            else:
                row["6"] = chain_verdict(
                    path,
                    completion,
                    Psig=flipped_curve(P[0], P[1]),
                    Qsig=flipped_curve(Q[0], Q[1]),
                    Esig=flipped_curve,
                )
        print(f"  6 -> {row['6']}", flush=True)

        # 7. translates by the nonzero points of E_sig[4], order 2 and order 4
        seven = item7(path, Esig, P, Q, completion)
        row["7"] = seven
        print(f"  7 -> {len(seven)} cases, ok={seven.count('Ok') if False else sum(1 for item in seven if item.endswith(':Ok'))}", flush=True)

        # 8. the old doubled-twice points
        row["8"] = chain_verdict(path, completion, Psig=4 * P, Qsig=4 * Q)
        print(f"  8 -> {row['8']}", flush=True)

        # 9. odd scalars that preserve order
        nine = {}
        for scalar in (ZZ(3), ZZ(1 << (e - 1)) + 1):
            left, right = normalize_joint(scalar * P, scalar * Q)
            nine[str(scalar)] = chain_verdict(path, completion, Psig=left, Qsig=right)
        row["9"] = nine
        print(f"  9 -> {nine}", flush=True)

        # 10. the other Montgomery models, sign re-normalized
        ten = []
        transported = other_models(A_sig, P, Q)
        normalized_models = []
        for curve, coefficient, raw_left, raw_right in transported:
            left, right = normalize_joint(raw_left, raw_right)
            normalized_models.append((curve, coefficient, left, right))
            if is_least_model(coefficient):
                ten.append("LEAST")
            elif left[1] == 0:
                ten.append("Sign")
            else:
                ten.append("Model")
        row["10"] = ten
        print(f"  10 -> {ten}", flush=True)

        # 11. different message, at a salt whose degree is prime
        other_message = message + "-other"
        prime_salt, scalar = first_prime_salt(level, key, other_message)
        if prime_salt is None:
            row["11"] = "MISSING"
        else:
            degree = (1 << (2 * e_prime - 1)) - scalar**2
            row["11"] = chain_verdict(path, completion, s=scalar, q=degree)
        print(f"  11 -> {row['11']} salt {prime_salt}", flush=True)

        # 12. the other key, at a salt whose degree is prime for that key
        sibling = load_vector(ROOT / "prism-verify" / "testdata" / SIBLING[name])
        prime_salt, scalar = first_prime_salt(level, sibling["A"], message)
        if prime_salt is None:
            row["12"] = "MISSING"
        else:
            A_other = fp2_from_hex(sibling["A"], Fp2, fp_len)
            other_curve = montgomery(A_other)
            other_p = point(
                other_curve,
                fp2_from_hex(sibling["VkPx"], Fp2, fp_len),
                fp2_from_hex(sibling["VkPy"], Fp2, fp_len),
            )
            other_q = point(
                other_curve,
                fp2_from_hex(sibling["VkQx"], Fp2, fp_len),
                fp2_from_hex(sibling["VkQy"], Fp2, fp_len),
            )
            degree = (1 << (2 * e_prime - 1)) - scalar**2
            row["12"] = chain_verdict(
                path, completion, s=scalar, q=degree, P1=other_p, P2=other_q
            )
        print(f"  12 -> {row['12']} salt {prime_salt}", flush=True)

        # 13. row 10 with Model off. Joint normalization. All five must accept.
        thirteen_models = []
        for curve, coefficient, left, right in normalized_models:
            if left[1] == 0:
                thirteen_models.append("Sign")
                continue
            verdict = chain_verdict(
                path,
                completion,
                Psig=left,
                Qsig=right,
                Esig=curve,
                s=ZZ(rows["s"]),
                q=ZZ(rows["q"]),
            )
            thirteen_models.append(verdict)
        seven_ok = any(item.endswith(":Ok") for item in seven)
        row["13"] = {
            "item7_reaches_ok": seven_ok,
            "item10_without_model": thirteen_models,
            "all_ok": thirteen_models == ["Ok"] * 5,
        }
        print(f"  13 -> item7_ok={seven_ok} item10={thirteen_models}", flush=True)

        # 14. determinant-one transforms of the signature pair
        fourteen = {}
        for label, raw_left, raw_right in (
            ("Q+P", P, Q + P),
            ("P+Q", P + Q, Q),
            ("Q,-P", Q, -P),
        ):
            left, right = normalize_joint(raw_left, raw_right)
            fourteen[label] = chain_verdict(path, completion, Psig=left, Qsig=right)
        row["14"] = fourteen
        print(f"  14 -> {fourteen}", flush=True)
        report[name] = row
        if os.environ.get("S5_COMPARE") != "1":
            (OUT / "negatives.json").write_text(json.dumps(report, indent=2) + "\n")
        if os.environ.get("S5_COMPARE") != "1" and not row["13"]["all_ok"]:
            fail(
                f"{name} item 13",
                f"a transported model did not accept: {thirteen_models}",
            )

    if os.environ.get("S5_COMPARE") != "1":
        (OUT / "negatives.json").write_text(json.dumps(report, indent=2) + "\n")
        print("wrote negatives.json", flush=True)
    return report


def honest_vectors():
    paths = list(VECTORS)
    accept = ROOT / "prism-verify" / "testdata" / "accept"
    paths.extend(sorted(accept.glob("s5_*.txt")))
    return paths


def compare_completions():
    """Chain verdicts of the closed form against the library completion.

    A difference stops the run. Known-answer files are not written.
    """
    os.environ["S5_COMPARE"] = "1"
    paths = honest_vectors()
    differ = []
    for path in paths:
        library = run_vector(path, run_chains=True, completion="library")
        closed = run_vector(path, run_chains=True, completion="closed")
        mark = "same" if library == closed else "DIFFER"
        print(f"CMP {path.name} library={library} closed={closed} {mark}", flush=True)
        if library != closed:
            differ.append(path.name)
            break
    if differ:
        print(f"STOPPED honest difference: {differ}", flush=True)
        sys.exit(1)
    for path in VECTORS:
        library = run_negatives_one(path, "library")
        closed = run_negatives_one(path, "closed")
        if library != closed:
            print(f"STOPPED negatives {path.name}", flush=True)
            print("library", json.dumps(library, indent=2), flush=True)
            print("closed", json.dumps(closed, indent=2), flush=True)
            sys.exit(1)
        print(f"CMP negatives {path.name} same", flush=True)
    print(
        f"CMP identical {len(paths)} honest vectors and {len(VECTORS)} negative files",
        flush=True,
    )


def run_negatives_one(path, completion):
    saved = list(VECTORS)
    try:
        VECTORS[:] = [path]
        return run_negatives(completion)
    finally:
        VECTORS[:] = saved


def main():
    if os.environ.get("S5_COMPARE") == "1":
        compare_completions()
        return
    if os.environ.get("S5_NEGATIVES") == "1":
        run_negatives()
        return
    only = os.environ.get("S5_CHAIN_VECTOR")
    if only is not None:
        path = VECTORS[int(only)]
        print(f"=== chains {path.name} ===", flush=True)
        run_vector(path, run_chains=True)
        return

    # First pass: Weil + algebra on all vectors (fast)
    for path in VECTORS:
        print(f"=== algebra {path.name} ===", flush=True)
        run_vector(path, run_chains=False)

    # Chains: start with NIST-I primary vector (slow)
    print("=== chains s5_lvl1.txt ===", flush=True)
    run_vector(VECTORS[0], run_chains=True)


if os.environ.get("S5_IMPORT") != "1":
    main()
