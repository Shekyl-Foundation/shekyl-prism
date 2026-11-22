# Source audit (before modelling)

Reference: mariascrs/PRISM_v2 at
`87fad7da5f12f79112521677f4f5530672afe9ba` (repository submodule pin).

- `src/signature/ref/lvlx/sign.c:48–65`: reference samples a **composite**
  response norm using RepresentInteger, intersects with the secret ideal,
  and calls `dim2id2iso_arbitrary_isogeny_evaluation`.
- shekyl-prism `docs/SIGNER_SPEC.md:60–72`: our Section 5 protocol uses a
  **prime** response norm. RepresentInteger is NOT on our signing path.
  The shared secret-touching norm equation is **quat_qlapoti**. Do not
  mistake a RepresentInteger experiment for a signing experiment here.
- `src/id2iso/ref/lvlx/dim2id2iso.c:298–307`: calls `quat_qlapoti` with
  17-bit generator bound, 20,000 multiples, exponent e−2, and the
  extended-Cornacchia parameters (q=1, bad-prime product=1, 101 primes
  2 or 1 mod 4, 32 primality rounds).
- `src/quaternion/ref/generic/qlapoti.c:511–567`: first maps the input to
  a **shortest equivalent ideal**, then reduces that basis. Thus the N
  inside this search is NOT necessarily the input secret ideal's norm.
  Measure both input bits(N_sec) and the post-reduction small norm.
- `qlapoti.c:50–100,571–700`: 400,000 passes. On a new generator, sample four
  bounded coefficients until the scalar part is invertible modulo n
  and gcd(nrd(alpha),n²)=n. On reuse, advance lambda until coprime to n.
  Reuse requires gcd(2a,2b)=1 and the second reduced 2-D vector shorter
  than sqrt(p). A failed mod/Cornacchia test keeps an eligible generator;
  an unsuitable successful output resets it. Reset also every 20,000
  passes. m<0 merely continues (does not reset the generator).
- `qlapoti.c:327–430,454–510,735–790`: nearest-plane vector in the reduced 2-D
  lattice produces a target M4. Check residue conditions, solve M4 as
  a sum of two squares, then check the output quaternion's end type or
  odd-norm shape. Those postconditions matter to the success rate.
- `integers.c:47–110,192–275`: trial-factor M4 by the 101 good primes; the
  residual must be 1 or a probable prime with −1 a quadratic residue.
  Extra data-dependent loops include valuations, Cornacchia's Euclidean
  descent, and complex powering, besides the two uncapped outer helpers.
- `lll/lll_applications.c:48–126`: keygen samples coefficients uniformly
  from [−64,64]^4 on a reduced basis; tests the quotient norm for primality;
  cap 129^4 = 276,922,881 samples. `keygen.c:29–40` also has an uncapped
  outer retry around sampling, prime-norm equivalence, and translation.

## A reference sampler detail that must not be silently fixed

`intbig.c:555–570`, `ibz_rand_interval_bits(rand,m)`, draws uniformly from
[−2^m,2^m] and then **subtracts m**, not 2^m. At m=17 its actual support
is [−131089,131055]. This model preserves that support. It is a research
observation, not a production patch.

## Scope of the model

Use the actual maximal order O0 = <1,i,(i+j)/2,(1+k)/2>, not the old
index-4 order; prime response ideals; simulated keys with varied input
norm lengths. Port search arithmetic, resets, residue and postconditions.
Use PARI reduction for setup rather than claim an exact C random-tape
replay. Cross-check algebraic output norm identities. A full-schedule
search must also bound **every inner operation**, not just pad outer
passes. Any residual variable-cost primitive will be named explicitly.
