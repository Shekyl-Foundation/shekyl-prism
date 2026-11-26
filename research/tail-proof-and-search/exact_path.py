import math,sys
from fractions import Fraction as F
def run(lam, ex, ey, verbose=True):
    n=0
    while True:
        r = lam+ex*ex+ey*ey
        if verbose: print(f" state lam={float(lam):.6g} eps=({float(ex):.6g},{float(ey):.6g}) r={float(r):.12g} r-1={float(r-1):.3g}")
        if r>=1: return n
        n+=1
        lam = lam/(r*r); qx = ex/r; qy = -ey/r
        mx = math.floor(qx+F(1,2)); my = math.floor(qy+F(1,2))
        if verbose: print(f"   swap {n}: q=({float(qx):.9g},{float(qy):.9g}) m=({mx},{my})")
        ex, ey = qx-mx, qy-my
for lam,ey in [(0.0004891198393349568,-0.30657345025948896),(0.00391647422708101,-0.2953298498417113)]:
    print(run(F(lam),F(-1,2),F(ey)))
