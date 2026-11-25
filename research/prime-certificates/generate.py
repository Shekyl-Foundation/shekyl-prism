#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Generate pinned primality certificates for the signer's fixed primes.

Needs PARI/GP (this box uses the micromamba Sage environment, PARI 2.17.4)
to produce Atkin–Morain ECPP certificates, and the sibling verify.py to
search Lucas N+1 parameters for the field primes. Optional N-1 certificates
are attempted with a timeout; they are recorded only when they succeed.

Usage, from the repository root, with `gp` on PATH:

    python3 research/prime-certificates/generate.py
    python3 research/prime-certificates/generate.py --nm1-timeout 120

This overwrites files under research/prime-certificates/certs/.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
CERTS = HERE / "certs"
sys.path.insert(0, str(HERE))
import verify  # noqa: E402

PARI_THREADS = 2
PARI_STACK = 1 << 30  # 1 GiB


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def to_gp(obj) -> str:
    if isinstance(obj, int):
        return str(obj)
    if isinstance(obj, list):
        return "[" + ", ".join(to_gp(x) for x in obj) + "]"
    raise TypeError(type(obj))


def pari_to_python(g):
    typ = g.type()
    if typ == "t_INT":
        return int(g)
    if typ in ("t_VEC", "t_COL", "t_LIST"):
        return [pari_to_python(g[i]) for i in range(len(g))]
    raise TypeError(typ)


def stringify(obj):
    if isinstance(obj, int):
        return str(obj)
    if isinstance(obj, list):
        return [stringify(x) for x in obj]
    return obj


def steps_as_dicts(steps):
    out = []
    for s in steps:
        out.append({"N": str(s[0]), "t": str(s[1]), "s": str(s[2]), "a": str(s[3]), "P": [str(s[4][0]), str(s[4][1])]})
    return out


def find_lucas_np1(n: int, factors: dict) -> dict:
    """Search (P, Q) for a Lucas N+1 certificate of n."""
    # Prefer small Q in {-1, 1, ±2, ±3}, then a slightly wider sweep.
    qs = [-1, 1, 2, -2, 3, -3, 4, -4, 5, -5]
    for q in qs:
        for p in range(1, 500):
            d = p * p - 4 * q
            if d == 0:
                continue
            try:
                if verify.ijacobi(d, n) != -1:
                    continue
            except ValueError:
                continue
            cert = {"P": p, "Q": q, "D": d, "np1_factors": {str(k): int(v) for k, v in factors.items()}}
            try:
                verify.verify_lucas_np1(n, cert)
                return cert
            except ValueError:
                continue
    raise RuntimeError(f"no Lucas N+1 parameters found for {n.bit_length()}-bit N")


def try_nm1(n: int, timeout: float) -> tuple[object | None, float, str]:
    """Attempt a PARI N-1 certificate in a subprocess, with a hard timeout."""
    if timeout <= 0:
        return None, 0.0, "skipped"
    script = f"""
default(parisizemax, "1G");
default(nbthreads, {PARI_THREADS});
N = {n};
t = gettime();
c = primecert(N, 1);
print("TIME ", gettime() - t);
if (c == 0, print("RESULT 0"); quit);
if (type(c) == "t_INT", print("BARE ", c); quit);
print("CERT ", c);
quit
"""
    t0 = time.perf_counter()
    try:
        proc = subprocess.run(
            ["gp", "-q"],
            input=script,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None, time.perf_counter() - t0, f"timeout after {timeout}s"
    except FileNotFoundError:
        return None, 0.0, "gp not on PATH"
    dt = time.perf_counter() - t0
    out = proc.stdout
    err = proc.stderr
    if proc.returncode != 0 or "CERT " not in out and "BARE " not in out:
        reason = "gp failed"
        tail = (err or out).strip().splitlines()
        if tail:
            reason = tail[-1][:200]
        return None, dt, reason
    if "BARE " in out:
        bare = int(out.strip().split("BARE ")[-1].split()[0])
        return {"format": "pari-nm1", "N": str(n), "certificate": bare}, dt, "ok"
    raw = out.strip().split("CERT ")[-1].strip()
    # PARI prints a vector; evaluate only after a tight syntax check.
    try:
        tree = parse_pari_vector(raw)
    except (ValueError, SyntaxError) as exc:
        return None, dt, f"could not parse N-1 output: {exc}"
    return {"format": "pari-nm1", "N": str(n), "certificate": stringify(tree)}, dt, "ok"


def parse_pari_vector(text: str):
    """Parse a PARI vector of integers (no identifiers) into Python lists."""
    cleaned = []
    for ch in text:
        if ch in " \n\t\\\r":
            continue
        cleaned.append(ch)
    s = "".join(cleaned)
    if not s:
        raise ValueError("empty")
    # Safe: only digits, minus, commas, brackets.
    allowed = set("0123456789-,[]")
    if any(ch not in allowed for ch in s):
        raise ValueError("unexpected character in PARI vector")
    return json.loads(s.replace("[", "[").replace("]", "]"))  # already JSON-like


def generate(nm1_timeout: float) -> int:
    try:
        import cypari2
    except ImportError:
        print("generate.py needs cypari2 (the Sage/PARI Python bindings) and gp on PATH", file=sys.stderr)
        return 1

    pari = cypari2.Pari()
    pari.default("parisizemax", PARI_STACK)
    pari.default("nbthreads", PARI_THREADS)
    pari_ver = str(pari.version())
    # pari.version() may be a vector; also read the banner.
    version_str = subprocess.run(["gp", "-q", "--version"], text=True, capture_output=True, check=False).stdout.strip()
    print(f"PARI: {version_str or pari_ver}")
    print(f"threads: {PARI_THREADS}  backend for N+1: {verify.USING}")

    inventory = verify.load_primes()
    CERTS.mkdir(parents=True, exist_ok=True)
    records = []
    for entry in inventory["large_primes"]:
        pid = entry["id"]
        n = int(entry["value_dec"])
        print(f"\n== {pid}  {entry['bit_length']} bits  {entry['formula']}")
        rec = {
            "id": pid,
            "kind": entry["kind"],
            "sets": entry["sets"],
            "bit_length": entry["bit_length"],
            "formula": entry["formula"],
            "N": str(n),
            "N_sha256_dec": entry["sha256_dec"],
        }

        t0 = time.perf_counter()
        cert = pari.primecert(n)
        ecpp_s = time.perf_counter() - t0
        if cert == 0:
            print(f"FAIL: PARI primecert returned 0 for {pid}", file=sys.stderr)
            return 1
        valid = int(pari.primecertisvalid(cert)) if cert.type() != "t_INT" or int(cert) != n else 1
        # Bare integers: primecertisvalid may want a vector; treat N < 2^64 as ok if isprime.
        if cert.type() == "t_INT":
            valid = 1 if int(cert) == n and n < (1 << 64) else 0
        if not valid and cert.type() != "t_INT":
            # primecertisvalid on a full cert
            print(f"FAIL: PARI primecertisvalid rejected its own cert for {pid}", file=sys.stderr)
            return 1
        py = pari_to_python(cert)
        if isinstance(py, int):
            steps_json = py
            gp_body = str(py)
            n_steps = 0
        else:
            steps_json = steps_as_dicts(py)
            gp_body = to_gp(py) + "\n"
            n_steps = len(py)

        rel_json = f"certs/{pid}.ecpp.json"
        rel_gp = f"certs/{pid}.ecpp.gp"
        path_json = HERE / rel_json
        path_gp = HERE / rel_gp
        payload = {
            "format": "pari-ecpp-2.17",
            "pari_version": version_str or "2.17.4",
            "id": pid,
            "N": str(n),
            "steps": steps_json,
        }
        path_json.write_text(json.dumps(payload, indent=2) + "\n")
        path_gp.write_text(gp_body if gp_body.endswith("\n") else gp_body + "\n")

        primo_rel = None
        primo_s = None
        if not isinstance(py, int):
            t1 = time.perf_counter()
            primo = str(pari.primecertexport(cert, 1))
            primo_s = time.perf_counter() - t1
            primo_rel = f"certs/{pid}.primo.txt"
            (HERE / primo_rel).write_text(primo if primo.endswith("\n") else primo + "\n")

        # Independent verification immediately.
        leaves = []
        verify.verify_ecpp(n, steps_json, leaves)
        print(f"  ECPP: {ecpp_s:.3f}s, {n_steps} steps, leaf={leaves[-1] if leaves else n}, independent verify ok")

        rec["ecpp_json"] = rel_json
        rec["ecpp_gp"] = rel_gp
        rec["ecpp_seconds"] = round(ecpp_s, 6)
        rec["ecpp_steps"] = n_steps
        rec["ecpp_leaf"] = str(leaves[-1] if leaves else n)
        rec["ecpp_json_sha256"] = sha256_file(path_json)
        rec["ecpp_gp_sha256"] = sha256_file(path_gp)
        if primo_rel:
            rec["primo"] = primo_rel
            rec["primo_sha256"] = sha256_file(HERE / primo_rel)
            rec["primo_export_seconds"] = round(primo_s or 0.0, 6)
            rec["primo_format"] = "Primo format 4 (export of the same ECPP proof; not an independent method)"

        if entry["kind"] == "field_prime":
            t1 = time.perf_counter()
            np1 = find_lucas_np1(n, entry["np1_factorization"])
            np1_s = time.perf_counter() - t1
            np1_doc = {
                "format": "lucas-np1",
                "id": pid,
                "N": str(n),
                "P": np1["P"],
                "Q": np1["Q"],
                "D": np1["D"],
                "np1_factors": np1["np1_factors"],
                "theorem": "If N+1 is completely factored and a Lucas sequence U_k(P,Q) with D=P^2-4Q satisfies (D/N)=-1, U_{N+1}≡0 (mod N), and gcd(U_{(N+1)/q}, N)=1 for every prime q dividing N+1, then N is prime.",
            }
            rel = f"certs/{pid}.np1.json"
            (HERE / rel).write_text(json.dumps(np1_doc, indent=2) + "\n")
            verify.verify_lucas_np1(n, np1_doc)
            rec["np1_json"] = rel
            rec["np1_seconds"] = round(np1_s, 6)
            rec["np1_P"] = np1["P"]
            rec["np1_Q"] = np1["Q"]
            rec["np1_json_sha256"] = sha256_file(HERE / rel)
            print(f"  N+1 Lucas: {np1_s:.3f}s, P={np1['P']}, Q={np1['Q']}, D={np1['D']}, independent verify ok")

        nm1, nm1_s, nm1_why = try_nm1(n, nm1_timeout)
        rec["nm1_attempt_seconds"] = round(nm1_s, 6)
        rec["nm1_status"] = nm1_why
        if nm1 is not None:
            try:
                verify.verify_nm1(n, nm1)
                rel = f"certs/{pid}.nm1.json"
                (HERE / rel).write_text(json.dumps(nm1, indent=2) + "\n")
                rec["nm1_json"] = rel
                rec["nm1_json_sha256"] = sha256_file(HERE / rel)
                rec["nm1_seconds"] = round(nm1_s, 6)
                print(f"  N-1 Pocklington–Lehmer: {nm1_s:.3f}s, independent verify ok")
            except ValueError as exc:
                rec["nm1_status"] = f"produced but independent verifier refused: {exc}"
                print(f"  N-1 produced but refused by independent verifier: {exc}")
        else:
            print(f"  N-1: {nm1_why} ({nm1_s:.3f}s)")

        records.append(rec)

    manifest = {
        "title": "Primality certificates for the shekyl-prism fixed primes",
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "pari": version_str or "2.17.4",
        "pari_threads": PARI_THREADS,
        "inventory": "primes.json",
        "base_commit": inventory.get("base_commit"),
        "note": "ECPP certificates were produced by PARI primecert (Atkin–Morain). They are checked by research/prime-certificates/verify.py, which does not call primecertisvalid. Primo format 4 files are a re-encoding of the same ECPP proof, not a second method. Lucas N+1 certificates for the field primes are a second, independent method. An N-1 certificate is included only when PARI produced one in the allotted time and the independent verifier accepted it.",
        "certificates": records,
    }
    man_path = CERTS / "manifest.json"
    man_path.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"\nwrote {man_path.relative_to(HERE.parent.parent)}")
    print("re-run: python3 research/prime-certificates/verify.py")
    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("--nm1-timeout", type=float, default=90.0, help="seconds per prime for a PARI N-1 attempt (0 to skip)")
    args = parser.parse_args(argv)
    return generate(nm1_timeout=args.nm1_timeout)


if __name__ == "__main__":
    sys.exit(main())
