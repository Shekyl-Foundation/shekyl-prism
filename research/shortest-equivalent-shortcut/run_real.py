import sys, json, random, time
from check_lib import *


def val(t): return int(t, 16)
FX = "/tmp/dev_fx"
SIGFX = "../signing-intersection-oracle/fixtures"
res = {}
allfails = []
rng = random.Random(20261126)
sets = sys.argv[1:] or list(SETS)
for name in sets:
    f, e, p, L, B = params(name)
    tot = {}; src = {}
    def run(src_name, N, u, v, tag, twists=3):
        rec, fails = check_basis(p, N, u, v, tag)
        add(tot, rec); add(src.setdefault(src_name, {}), rec)
        allfails.extend(fails)
        # reduced version
        ru, rv, _, _ = lagrange(p, u, v)
        rec, fails = check_basis(p, N, ru, rv, tag + "/red")
        add(tot, rec); add(src.setdefault(src_name + "/reduced", {}), rec)
        allfails.extend(fails)
        for t in range(twists):
            tu, tv = unimodular(rng, p, u, v, kmax=3, bits=(0 if t < 2 else 40))
            rec, fails = check_basis(p, N, tu, tv, tag + f"/tw{t}")
            add(tot, rec); add(src.setdefault(src_name + "/twisted", {}), rec)
            allfails.extend(fails)
            tu, tv = unimodular(rng, p, ru, rv, kmax=3, bits=0)
            rec, fails = check_basis(p, N, tu, tv, tag + f"/rtw{t}")
            add(tot, rec); add(src.setdefault(src_name + "/reduced-twisted", {}), rec)
            allfails.extend(fails)
    # (1) dev vectors: 11 per set (55 total)
    for i, l in enumerate(open(f"{FX}/vectors/{name}.txt")):
        r = l.split()
        run("dev-vectors", val(r[1]), tuple(val(x) for x in r[2:6]), tuple(val(x) for x in r[6:10]), f"{name}:vec{i}:{r[0]}")
    # (2) dev ideals: admitted keys n xn yn u v s xq yq
    for i, l in enumerate(open(f"{FX}/ideals/{name}.txt")):
        r = l.split()
        run("dev-ideals", val(r[0]), tuple(val(x) for x in r[3:7]), tuple(val(x) for x in r[7:11]), f"{name}:ideal{i}")
    # (3) signing-intersection-oracle fixtures: U,V,T of ok rows
    for l in open(f"{SIGFX}/{name}.tsv"):
        c = l.rstrip("\n").split("\t")
        if c[1] != "ok": continue
        vals = [val(x) for x in c[3].split()]
        run("oracle-fixtures", vals[8], tuple(vals[0:4]), tuple(vals[4:8]), f"{name}:{c[0]}", twists=1)
    res[name] = dict(total=tot, by_source=src)
    print(name, json.dumps(tot), flush=True)
json.dump(res, open(f"results/real_{'_'.join(sets)}.json", "w"), indent=1)
print("FAILS", len(allfails))
for f in allfails[:40]: print(f)
