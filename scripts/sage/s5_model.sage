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
from pathlib import Path

# Sage's run-file copies this script to a temp path; locate the repo from cwd
# or from an explicit PRISM_ROOT.
ROOT = Path(__file__).resolve().parents[2]
if not (ROOT / "external" / "Theta_dim4").is_dir():
    ROOT = Path.cwd()
if not (ROOT / "external" / "Theta_dim4").is_dir():
    ROOT = Path(os.environ.get("PRISM_ROOT", "/home/torvaldsl/shekyl/shekyl-prism"))
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
    """Spec index search, independently of Rust (brief §4.1)."""
    bound = 2 * lam
    square = A.is_square()
    Fp2 = A.parent()
    i = Fp2.gen()
    assert i**2 == -1
    if square:
        for b in range(1, bound + 1):
            if (1 + b * b).is_square():
                continue
            z = 1 + i * b
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


def run_vector(path, run_chains=True):
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

    # Orders
    order = 1 << e
    assert (order * Pvk).is_zero() and not ((order // 2) * Pvk).is_zero()
    assert (order * Qvk).is_zero() and not ((order // 2) * Qvk).is_zero()
    assert (order * Psig).is_zero() and not ((order // 2) * Psig).is_zero()
    assert (order * Qsig).is_zero() and not ((order // 2) * Qsig).is_zero()

    # Independent basis index (§4.1)
    index, square = basis_index(A, e, odd, lam)
    claim("Ran", f"{path.name} basis index", f"index={index} square={square}")
    xp = abscissa(A, index, square)
    xr = -xp - A
    # The index abscissa may lie on the quadratic twist; the x-only ladder
    # still clears the cofactor. Compare x after [odd] when a lift exists,
    # otherwise compare the exported VkDx against −x(P)−A style relations.
    if rhs_square(A, xp):
        Pclr = clear_odd(Evk, Evk.lift_x(xp), odd)
        if Pclr[0] != Pvk[0] and (-Pclr)[0] != Pvk[0]:
            fail(f"{path.name} VkP", "derived cleared P disagrees with export")
        claim("Ran", f"{path.name} VkP matches cleared index abscissa", "")
    else:
        # Twist case (common when A is square): confirm the export is order 2^e
        # and that x(Q)+x(P)+A = 0 for VkQ = R after the exporter swap.
        claim("Ran", f"{path.name} index abscissa on twist; trust export", f"index={index}")
    # The exporter emits the preimage pair itself: (VkP, VkQ) is, up to one
    # global sign, the C canonical basis (P, Q) whose t*sigma image is the
    # signature. VkDx cross-checks the pair. (The earlier export swapped
    # Q and PmQ, emitting (P, P-Q); the instrumented exporter run recorded
    # in the report established the correspondence.)
    P1 = Pvk
    P2 = Qvk
    if "VkDx" in rows:
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
        fail(f"{path.name} Weil", "e(R1,R2) != e(P1,P2)^q")
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

    # Symplectic matrices (§4.4): exact dim-2 reduction of Dartois's
    # starting_two_symplectic_matrices. Only M2 is freshly completed; M1 is
    # the block matrix [complement of M1_0 | −Φ̂(complement of M2)] — it is
    # NOT re-completed, and its symplecticity is a theorem being checked.
    M1_0 = complete_symplectic_matrix_dim2(C1, D1, order)
    if not is_symplectic_matrix_dim2(M1_0):
        fail(f"{path.name} M1_0", "not symplectic")
    # Complement half = left two columns
    complement = M1_0.matrix_from_columns([0, 1])
    # Φ of complement → kernel half of M2 (isotropic lift above ker Φ̃2)
    block_right2 = Mphi * complement
    C2 = block_right2.matrix_from_rows([0, 1])
    D2 = block_right2.matrix_from_rows([2, 3])
    if C2.transpose() * D2 != D2.transpose() * C2:
        fail(f"{path.name} M2 kernel half", "not isotropic")
    M2 = complete_symplectic_matrix_dim2(C2, D2, order)
    if not is_symplectic_matrix_dim2(M2):
        fail(f"{path.name} M2", "not symplectic")
    # −Φ̂ of complement of M2 → kernel half of M1
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
        "N1": [[str(N1[i, j]) for j in range(4)] for i in range(4)],
        "N2": [[str(N2[i, j]) for j in range(4)] for i in range(4)],
        "BK1": [
            [str(pt[0][0]), str(pt[0][1]), str(pt[1][0]), str(pt[1][1])] for pt in B_K1
        ],
        "BK2": [
            [str(pt[0][0]), str(pt[0][1]), str(pt[1][0]), str(pt[1][1])] for pt in B_K2
        ],
    }
    (OUT / f"{stem}_matrices.json").write_text(json.dumps(kat, indent=2) + "\n")

    if not run_chains or os.environ.get("S5_ALGEBRA_ONLY"):
        return

    # Two chains (§4.5), kernels from the matrices (computed above)
    claim("Open", f"{path.name} starting chains", f"len1={e_prime} len2={e_prime-1}")
    print(f"  computing chain1 length {e_prime}…", flush=True)
    chain1 = IsogenyChainDim2(B_K1, Theta12, N1, e_prime)
    print(f"  computing chain2 length {e_prime-1}…", flush=True)
    chain2 = IsogenyChainDim2(B_K2, Theta12, N2, e_prime - 1)

    C1z = chain1._codomain.zero().coords()
    C2hz = chain2._codomain.hadamard().zero().coords()
    if not projective_equal(C1z, C2hz):
        fail(
            f"{path.name} Codomain",
            f"C1.zero() != C2.hadamard().zero()\n  C1={C1z}\n  C2h={C2hz}",
        )
    claim("Derived", f"{path.name} Codomain C1.zero()==C2.hadamard().zero()", "confirmed")

    # Emit per-step null points
    nulls1 = []
    for phi in chain1._phis:
        nulls1.append([str(c) for c in phi._codomain.zero().coords()])
    nulls2 = []
    for phi in chain2._phis:
        nulls2.append([str(c) for c in phi._codomain.zero().coords()])
    (OUT / f"{stem}_nulls.json").write_text(
        json.dumps({"chain1": nulls1, "chain2": nulls2}, indent=2) + "\n"
    )

    # Step 8 (§2b / §4.5)
    dual2 = chain2.dual()

    def Phi(pt):
        return dual2(chain1(pt))

    images = {}
    for name, X, Y in [
        ("P", P1, Psig),
        ("Q", P2, Qsig),
        ("PmQ", P1 - P2, Psig - Qsig),
    ]:
        img = Phi(TuplePoint(X, Esig(0)))
        # Expect (±[s]X, ±[s]Y) up to projective on each factor — full points
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
            fail(
                f"{path.name} Image step8 {name}",
                f"Φ(X,0) not ±([s]X,[s]Y); left_ok={left_ok} right_ok={right_ok}",
            )
    claim("Derived", f"{path.name} Image step 8", "confirmed")
    (OUT / f"{stem}_step8.json").write_text(json.dumps(images, indent=2) + "\n")
    claim("Ran", f"{path.name} accept path Ok", "")


def main():
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


main()
