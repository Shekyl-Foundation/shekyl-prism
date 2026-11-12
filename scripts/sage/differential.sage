# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Fresh Sage/Rust comparison, without rewriting committed answers.

Run from the repository root after building s5_challenge and verify_bytes:
  sage scripts/sage/differential.sage
Requires the two pinned submodules. Optional PRISM_DIFFERENTIAL_LEVEL is
NIST-I, NIST-III or NIST-V; absence runs all levels.
"""
import os
import runpy
import subprocess
import sys
import faulthandler
from pathlib import Path
from sage.all import GF, ZZ

os.environ["S5_IMPORT"] = "1"
os.environ["S5_COMPARE"] = "1"
m = runpy.run_path(str(Path.cwd() / "scripts/sage/s5_model.sage"))
root = m["ROOT"]
count = 0

# Each vector gets a fresh oracle interpreter and its own result denominator.
# Every child still executes the same algebra and all applicable mutations.
# No partial child run is counted as a complete comparison.
one = os.environ.get("PRISM_DIFFERENTIAL_ONE")
if one is None:
    total = 0
    paths = [p for p in m["honest_vectors"]()
             if os.environ.get("PRISM_DIFFERENTIAL_LEVEL", m["load_vector"](p)["level"]) == m["load_vector"](p)["level"]]
    if not paths:
        raise SystemExit("no vectors selected")
    for path in paths:
        env = dict(os.environ, PRISM_DIFFERENTIAL_ONE=path.name)
        result = subprocess.run([sys.executable, __file__], env=env, text=True, capture_output=True)
        print(result.stdout, end="", flush=True)
        if result.stderr:
            print(result.stderr, end="", file=sys.stderr, flush=True)
        if result.returncode:
            raise SystemExit(f"Sage child failed: {path.name}, code {result.returncode}")
        marks = [line for line in result.stdout.splitlines() if line.startswith("VECTOR_RESULT ")]
        if len(marks) != 1:
            raise SystemExit(f"missing child comparison denominator: {path.name}")
        total += int(marks[0].split()[2])
    print(f"Sage/Rust identical on {total} signatures and mutations across {len(paths)} isolated vectors", flush=True)
    raise SystemExit(0)
if one not in {p.name for p in m["honest_vectors"]()}:
    raise SystemExit("unknown isolated vector")
faulthandler.dump_traceback_later(120, repeat=True)


def encoded(z, width):
    re, im = m["parts"](z)
    return int(re).to_bytes(width, "little") + int(im).to_bytes(width, "little")


def compare(path, label, P=None, Q=None, curve=None, salt=None, message=None, sibling=None):
    global count
    rows = m["load_vector"](path)
    level = rows["level"]
    params = m["LEVELS"][level]
    e, width = params["e"], params["fp_len"]
    p = params["odd"] * (1 << e) - 1
    field = GF(p**2, name="i", modulus=ZZ["x"]("x^2+1"))
    ext = lambda name: m["fp2_from_hex"](rows[name], field, width)
    original_curve = m["montgomery"](ext("E"))
    curve = original_curve if curve is None else curve
    P = m["point"](original_curve, ext("Px"), ext("Py")) if P is None else P
    Q = m["point"](original_curve, ext("Qx"), ext("Qy")) if Q is None else Q
    salt = rows["salt"] if salt is None else salt
    message = rows["message"] if message is None else message
    key = rows["A"] if sibling is None else sibling["A"]
    s, prime = m["challenge_scalar"](level, key, message, salt)
    q = (1 << (2*(e-2)-1)) - s**2
    soft = {"Psig": P, "Qsig": Q, "Esig": curve, "s": s, "q": q}
    if sibling is not None:
        other = lambda name: m["fp2_from_hex"](sibling[name], field, width)
        vk = m["montgomery"](other("A"))
        soft.update(P1=vk(other("VkPx"), other("VkPy")), P2=vk(other("VkQx"), other("VkQy")))
    a = curve.a_invariants()[1]
    if not m["canonical_sign"](P[1]):
        expected = "Sign"
    elif a**2 == 4:
        expected = "Singular"
    elif a == 0 or a**2 == 3 or 2*a**2 == 9 or not m["is_least_model"](a):
        expected = "Model"
    elif prime != "prime":
        expected = "Composite"
    else:
        expected = m["run_vector"](path, run_chains=True, soft=soft, completion="closed")
    signature = b"".join(encoded(z, width) for z in [a,P[0],P[1],Q[0],Q[1]]) + bytes.fromhex(salt)
    record = f"{level} {key} {message.encode().hex() or '-'} {signature.hex()}\n"
    result = subprocess.run([str(m["rust_binary"]("verify_bytes"))],input=record,text=True,capture_output=True,check=True).stdout.strip()
    if result != expected:
        raise SystemExit(f"DIFFER {path.name} {label}: Sage={expected}, Rust={result}")
    count += 1
    print(f"MATCH {path.name} {label}: {result}", flush=True)


for path in m["honest_vectors"]():
    if path.name != one:
        continue
    rows = m["load_vector"](path)
    if os.environ.get("PRISM_DIFFERENTIAL_LEVEL", rows["level"]) != rows["level"]:
        continue
    # Independently execute all algebraic identities for each honest fixture.
    if m["run_vector"](path, run_chains=True, completion="closed") != "Ok":
        raise SystemExit(f"honest Sage failure: {path.name}")
    compare(path,"honest")

for path in m["VECTORS"]:
    if path.name != one:
        continue
    rows = m["load_vector"](path)
    level = rows["level"]
    if os.environ.get("PRISM_DIFFERENTIAL_LEVEL", level) != level:
        continue
    params = m["LEVELS"][level]
    e, width = params["e"], params["fp_len"]
    p = params["odd"] * (1 << e) - 1
    field = GF(p**2, name="i", modulus=ZZ["x"]("x^2+1"))
    ext = lambda name: m["fp2_from_hex"](rows[name],field,width)
    curve = m["montgomery"](ext("E"))
    P, Q = curve(ext("Px"),ext("Py")), curve(ext("Qx"),ext("Qy"))
    for label, left, right in [("joint sign",-P,-Q),("relative sign",P,-Q),("swap",Q,P),("shear Q+P",P,Q+P),("shear P+Q",P+Q,Q),("rotation",Q,-P),("scalar 3",3*P,3*Q),("high scalar",((1<<(e-1))+1)*P,((1<<(e-1))+1)*Q),("reduced order",4*P,4*Q)]:
        if label != "joint sign":
            left,right = m["normalize_joint"](left,right)
        compare(path,label,P=left,Q=right)
    for t in m["row7_translates"](curve):
        for label,left,right in [("P translate",P+t,Q),("Q translate",P,Q+t),("both translate",P+t,Q+t)]:
            left,right=m["normalize_joint"](left,right)
            compare(path,label,P=left,Q=right)
    for curve2,a,left,right in m["other_models"](ext("E"),P,Q):
        left,right=m["normalize_joint"](left,right)
        compare(path,"other model",P=left,Q=right,curve=curve2)
    for label,message in [("searched prime salt",rows["message"]),("new message",rows["message"]+"-other")]:
        salt,_=m["first_prime_salt"](level,rows["A"],message)
        if salt is None:
            raise SystemExit("prime salt search exhausted")
        compare(path,label,salt=salt,message=message)
    sibling=m["load_vector"](path.parent/m["SIBLING"][path.name])
    salt,_=m["first_prime_salt"](level,sibling["A"],rows["message"])
    if salt is None:
        raise SystemExit("other-key prime salt search exhausted")
    compare(path,"other key",salt=salt,sibling=sibling)

print(f"VECTOR_RESULT {one} {count}", flush=True)
