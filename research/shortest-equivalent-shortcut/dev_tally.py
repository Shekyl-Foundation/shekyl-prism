"""Rick's corpus shape: 55 dev vectors (11 per set), each of the returned (reduced) columns, 4 rotations of V by Gaussian units
(Rick's 'orientations') -- reproduced here as 4 unit twists of V, plus our beta = eps*u orientations."""
import json
from check_lib import *
def val(t): return int(t, 16)
tot = dict(rows=0, rot_checks=0, quotient_unit=0, quotient_real=0, quotient_pm1=0, even_N=0, lattice_eq=0, fails=0)
ev = {}
for name in SETS:
    f, e, p, L, B = params(name)
    for l in open(f"/tmp/dev_fx/vectors/{name}.txt"):
        r = l.split(); T = val(r[1])
        u = tuple(val(x) for x in r[2:6]); v = tuple(val(x) for x in r[6:10])
        ru, rv, _, _ = lagrange(p, u, v)       # returned raw columns (reduced)
        tot["rows"] += 1
        Np = None
        for k in range(4):
            vk = rv
            for _ in range(k): vk = J(vk)       # rotate V by the Gaussian unit i^k
            s = shortcut(p, T, ru, vk)
            tot["rot_checks"] += 1
            if "fail" in s: tot["fails"] += 1; continue
            tot["quotient_unit"] += 1
            tot["quotient_real"] += (s["e"][1] == 0); tot["quotient_pm1"] += (s["e"] in ((1,0),(-1,0)))
            Ld = direct(p, T, ru, vk, ru)
            tot["lattice_eq"] += (Ld == gaussian_lattice([s["Up"], s["Vp"]]))
            Np = s["Np"]
        tot["even_N"] += (Np % 2 == 0)
        ev.setdefault(name, []).append(int(Np % 2 == 0))
print(json.dumps(tot)); print(json.dumps(ev))
json.dump(dict(tot=tot, even_by_set=ev), open("results/dev_tally.json", "w"), indent=1)
