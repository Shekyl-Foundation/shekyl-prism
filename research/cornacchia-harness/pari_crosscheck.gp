\\ Copyright (c) 2025-2026, The Shekyl Foundation
\\
\\ All rights reserved.
\\ BSD-3-Clause

\\ PARI/GP-only check: Euclid-to-sqrt Cornacchia step count vs qfbcornacchia.
\\ Run: gp -q pari_crosscheck.gp
corn(M)={my(u=lift(sqrt(Mod(-1,M))),a,b,s=sqrtint(M),n=0);u=min(u,M-u);a=M;b=u;
  while(b>s,[a,b]=[b,a%b];n++);[b,sqrtint(M-b^2),n]};
{for(k=1,5,M=0;until(isprime(M)&&M%4==1,M=random(2^383)+2^383);
  r=corn(M);q=qfbcornacchia(1,M);
  print(r[1]^2+r[2]^2==M," ",vecsort([r[1],r[2]])==vecsort(q)," steps=",r[3]))}
quit
