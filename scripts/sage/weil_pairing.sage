# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Sage values of e(P_vk, Q_vk) and e([s]P_sig, [s]Q_sig) for the six
# original vectors. The encoding is the field's little-endian re || im,
# the same bytes prism-verify compares in its known-answer test.
# Run from the repo with the Sage environment on PATH:
#   sage scripts/sage/weil_pairing.sage

import os
from pathlib import Path

os.environ["S5_IMPORT"] = "1"
if "PRISM_ROOT" not in os.environ:
    os.environ["PRISM_ROOT"] = str(Path(__file__).resolve().parents[2])

load(os.environ["PRISM_ROOT"] + "/scripts/sage/s5_model.sage")


def enc(z, n):
    re, im = parts(z)
    return int(re).to_bytes(n, "little").hex() + int(im).to_bytes(n, "little").hex()


def main():
    lines = ["file\tvk\tsig"]
    for path in VECTORS:
        rows = load_vector(path)
        level = rows["level"]
        params = LEVELS[level]
        e = params["e"]
        fp_len = params["fp_len"]
        p = params["odd"] * (1 << e) - 1
        Fp2 = GF(p**2, name="i", modulus=ZZ["x"]("x^2+1"))
        Evk = montgomery(fp2_from_hex(rows["A"], Fp2, fp_len))
        Esig = montgomery(fp2_from_hex(rows["E"], Fp2, fp_len))
        Pvk = point(
            Evk,
            fp2_from_hex(rows["VkPx"], Fp2, fp_len),
            fp2_from_hex(rows["VkPy"], Fp2, fp_len),
        )
        Qvk = point(
            Evk,
            fp2_from_hex(rows["VkQx"], Fp2, fp_len),
            fp2_from_hex(rows["VkQy"], Fp2, fp_len),
        )
        Psig = point(
            Esig,
            fp2_from_hex(rows["Px"], Fp2, fp_len),
            fp2_from_hex(rows["Py"], Fp2, fp_len),
        )
        Qsig = point(
            Esig,
            fp2_from_hex(rows["Qx"], Fp2, fp_len),
            fp2_from_hex(rows["Qy"], Fp2, fp_len),
        )
        s = ZZ(rows["s"])
        order = 1 << e
        evk = Fp2(weil_pairing_pari(Pvk, Qvk, order))
        esig = Fp2(weil_pairing_pari(s * Psig, s * Qsig, order))
        lines.append(f"{path.name}\t{enc(evk, fp_len)}\t{enc(esig, fp_len)}")
    out = ROOT / "prism-verify" / "testdata" / "weil_pairing.tsv"
    out.write_text("\n".join(lines) + "\n")
    print(f"wrote {out.name} ({len(lines) - 1} vectors)", flush=True)


main()
