# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

import sys,time
sys.path.insert(0,'.')
from fne import *
from fsne import *
name=sys.argv[1]; _,e,f=SET_MAP[name]; p=prime_of(e,f); SEC=sec_degree(name)
sch=Sched(p.bit_length(), e, P=int(sys.argv[2]), K=int(sys.argv[3]))
print(sch.describe())
traces=set(); t0=time.time(); ok=0; eq=0; firsts=[]
for k in range(int(sys.argv[4])):
    rng=tape(b'fx',name,k); I0=random_prime_ideal(SEC,p,rng); Is,_=prime_norm_equivalent(I0,p,rng)
    while True:
        s=rng.getrandbits(e-4)|1; q=(1<<(2*e-5))-s*s
        if gmpy2.is_prime(q): break
    S=prepare_small(intersect_coprime(Is,random_prime_ideal(q,p,rng)),p)
    r1,i1=qlapoti_fixed(S,p,e-2,tape(b'fxt',name,k),sch)
    r2,i2=qlapoti_fixed(S,p,e-2,tape(b'fxt',name,k),sch,early_exit=True)
    traces.add(i1['trace']); ok+= r1 is not None; eq+= (r1==r2); firsts.append(i1['first'])
    if r1:
        (g1,d1),(g2,d2),_,_=r1
        assert nrd_num(g1,p)//d1**2//S.norm + nrd_num(g2,p)//d2**2//S.norm == 1<<(e-2)
print(name,'runs',k+1,'success',ok,'full==early',eq,'distinct traces',len(traces),'mean first',sum(firsts)/len(firsts),'time',round(time.time()-t0,1))
