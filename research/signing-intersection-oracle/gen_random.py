"""Large random corpus: random admitted bases (random N, shear within the guard, unit multiples),
random challenge scalars and root representations. usage: gen_random.py SET COUNT SEED"""
import random, sys, os
from oracle import *
from gen_corpus import *

def main(name, count, seed):
    f, e, p, L, B = params(name)
    rng = random.Random(f"rand-{name}-{seed}")
    out = []
    for i in range(count):
        kind = rng.random()
        if kind < 0.12:
            fac = sorted({rand_prime_bits(rng, B // 2 - 1 - rng.randrange(3)), rand_prime_bits(rng, B - B // 2 - 2 - rng.randrange(3))})
            if len(fac) < 2:
                continue
            rs, N = composite_root(rng, p, fac)
            tag = "composite"
        else:
            while True:
                N = rand_prime_bits(rng, rng.randrange(B - 5, B))
                if kind < 0.45 and N % 4 != 1:
                    continue
                break
            rs = secret_root(rng, p, N)
            tag = "prime-1mod4" if N % 4 == 1 else "prime"
        u, v = reduced_basis(p, N, rs)
        # random Gaussian shear of v by a multiple of u, kept inside the guard when possible
        g = (rng.randrange(-12, 13), rng.randrange(-12, 13))
        while g != (0, 0) and not guard_ok(p, N, shear(u, v, g)):
            g = (g[0] // 2, g[1] // 2)
        v2 = shear(u, v, g)
        units = [lambda w: w, lambda w: J(w), lambda w: tuple(-x for x in w), lambda w: J(J(J(w)))]
        u2, v2 = rng.choice(units)(u), rng.choice(units)(v2)
        s, q, rq = challenge(rng, p, e)
        rr = rng.randrange(3)
        if rr == 1:
            rq = (rq[0] + rng.randrange(-3, 4) * q, rq[1] + rng.randrange(-3, 4) * q)
        elif rr == 2:
            rq = ((-rq[0]), rq[1])
        out.append((f"random{i}-{tag}-g{g[0]}_{g[1]}", row(name, N, rs, u2, v2, s, rq)))
    os.makedirs("corpus_random", exist_ok=True)
    with open(f"corpus_random/{name}.tsv", "w") as fh:
        for c, r in out:
            fh.write(c + "\t" + r + "\n")
    print(name, len(out))

if __name__ == "__main__":
    main(sys.argv[1], int(sys.argv[2]), sys.argv[3])
