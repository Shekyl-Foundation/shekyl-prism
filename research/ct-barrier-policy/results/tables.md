## Recommendation table

asm-clean: control-flow trace identical across fixed, random and adversarial inputs under qemu (and correct outputs), no integer division; counted over the 4 build configs (`ok*` on aarch64 `c4` = trace equal; `udiv` is only SVE public-length `N/rdvl`). Taint: memcheck errors in the constant-time code, 5 sets, x86-64, config c1. dudect: max |t| at P248 / P500, x86-64, 1.97.1, c1 (threshold 4.5).

| candidate | unsafe where | toolchain | x86-64 asm-clean | aarch64 asm-clean | x86-64 taint | x86-64 dudect max\|t\| (P248 / P500) |
|---|---|---|---|---|---|---|
| `asm` | unsafe asm in the signer crate | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 25 shift) | 1.53 / 2.07 |
| `asm` | unsafe asm in the signer crate | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `crate` | unsafe asm, isolated in prism-ct-barrier | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 25 shift) | 2.18 / 1.80 |
| `crate` | unsafe asm, isolated in prism-ct-barrier | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `none` | safe (negative control) | 1.97.1 | 0/4 clean (branch: c1,c2,c3,c4) | 1/4 clean (branch: c1,c3,c4) | 81 errors | 32.36 / 2.10 |
| `none` | safe (negative control) | 1.85.1 | 0/2 clean (branch: c1,c3) | 0/2 clean (branch: c1,c3) | 55 errors | not run |
| `weak` | safe (negative control) | 1.97.1 | 0/4 clean (branch: c1,c2,c3,c4) | 1/4 clean (branch: c1,c3,c4) | 20 errors | 9.11 / 1.83 |
| `weak` | safe (negative control) | 1.85.1 | 0/2 clean (branch: c1,c3) | 0/2 clean (branch: c1,c3) | 14 errors | not run |
| `bbmask` | safe | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 24 shift) | 1.59 / 1.84 |
| `bbmask` | safe | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean; 6 shift) | not run |
| `bbuse` | safe | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 22 shift) | 1.62 / 2.05 |
| `bbuse` | safe | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `noinline` | safe | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 22 shift) | 2.31 / 2.12 |
| `noinline` | safe | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `selunpred` | safe (Rust >= 1.88) | 1.97.1 | 0/4 clean (branch: c1,c2,c3,c4) | 1/4 clean (branch: c1,c3,c4) | 87 errors | 30.00 / 2.79 |
| `selunpred` | safe (Rust >= 1.88) | 1.85.1 | n/a (does not build) | n/a (does not build) | build failed | not run |
| `subtle-bb` | safe (subtle, core_hint_black_box) | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 22 shift) | 1.85 / 2.94 |
| `subtle-bb` | safe (subtle, core_hint_black_box) | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean; 6 shift) | not run |
| `subtle-vol` | unsafe read_volatile inside subtle | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 18 shift) | 2.08 / 1.70 |
| `subtle-vol` | unsafe read_volatile inside subtle | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `volatile` | unsafe read_volatile in the signer crate | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 28 shift) | 2.44 / 1.72 |
| `volatile` | unsafe read_volatile in the signer crate | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `cmov` | unsafe asm inside cmov | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 21 shift) | 1.80 / 2.03 |
| `cmov` | unsafe asm inside cmov | 1.85.1 | 2/2 clean | 2/2 clean | 0 (clean) | not run |
| `ctutils` | unsafe asm inside cmov (via ctutils) | 1.97.1 | 4/4 clean | 4/4 clean | 0 (clean; 23 shift) | 2.54 / 1.57 |
| `ctutils` | unsafe asm inside cmov (via ctutils) | 1.85.1 | n/a (does not build) | n/a (does not build) | build failed | not run |

## Per configuration (trace verdict: ok / BR = secret-dependent control flow / build = does not compile)

- `c1`: opt 3, fat LTO, cgu 1
- `c2`: opt 2, fat LTO, cgu 1
- `c3`: opt 3, no LTO, cgu 16
- `c4`: opt 3, fat LTO, cgu 1, target-cpu (x86: x86-64-v3 = AVX2/BMI2/ADX; aarch64: neoverse-v1)

| candidate | 1.97.1 x86 c1 | 1.97.1 x86 c2 | 1.97.1 x86 c3 | 1.97.1 x86 c4 | 1.97.1 aar c1 | 1.97.1 aar c2 | 1.97.1 aar c3 | 1.97.1 aar c4 | 1.85.1 x86 c1 | 1.85.1 x86 c2 | 1.85.1 x86 c3 | 1.85.1 x86 c4 | 1.85.1 aar c1 | 1.85.1 aar c2 | 1.85.1 aar c3 | 1.85.1 aar c4 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `asm` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `crate` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `none` | BR | BR | BR | BR | BR | ok | BR | BR | BR | - | BR | - | BR | - | BR | - |
| `weak` | BR | BR | BR | BR | BR | ok | BR | BR | BR | - | BR | - | BR | - | BR | - |
| `bbmask` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `bbuse` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `noinline` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `selunpred` | BR | BR | BR | BR | BR | ok | BR | BR | build | build | build | build | build | build | build | build |
| `subtle-bb` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `subtle-vol` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `volatile` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `cmov` | ok | ok | ok | ok | ok | ok | ok | ok* | ok | - | ok | - | ok | - | ok | - |
| `ctutils` | ok | ok | ok | ok | ok | ok | ok | ok* | build | build | build | build | build | build | build | build |

## Static shape (config c1, sum over the five sets of zi::reduce + ct::exact_round)

| candidate | 1.97.1 x86 | 1.97.1 aar | 1.85.1 x86 | 1.85.1 aar |
|---|---|---|---|---|
| `asm` | br 1032 (reg 42), sel 9120, div 0 | br 934 (reg 9), sel 12760, div 0 | br 1046 (reg 114), sel 8960, div 0 | br 975 (reg 146), sel 12555, div 0 |
| `crate` | br 1032 (reg 42), sel 9120, div 0 | br 934 (reg 9), sel 12760, div 0 | br 1046 (reg 114), sel 8960, div 0 | br 975 (reg 146), sel 12555, div 0 |
| `none` | br 949 (reg 58), sel 9168, div 0 TRACE DIFF p248,p324,p376 | br 840 (reg 79), sel 12830, div 0 TRACE DIFF p248,p324,p376,p500 | br 953 (reg 132), sel 9056, div 0 TRACE DIFF p248,p376 | br 810 (reg 245), sel 12645, div 0 TRACE DIFF p248,p376,p500 |
| `weak` | br 1048 (reg 42), sel 9261, div 0 TRACE DIFF p248,p324,p376 | br 954 (reg 29), sel 12755, div 0 TRACE DIFF p248,p324,p376,p500 | br 1062 (reg 118), sel 9087, div 0 TRACE DIFF p248,p376 | br 996 (reg 167), sel 12642, div 0 TRACE DIFF p248,p376,p500 |
| `bbmask` | br 1032 (reg 41), sel 9150, div 0 | br 934 (reg 9), sel 12718, div 0 | br 1046 (reg 113), sel 8990, div 0 | br 975 (reg 146), sel 12571, div 0 |
| `bbuse` | br 1100 (reg 41), sel 9142, div 0 | br 974 (reg 10), sel 12710, div 0 | br 1106 (reg 113), sel 8976, div 0 | br 1018 (reg 148), sel 12563, div 0 |
| `noinline` | br 1032 (reg 42), sel 9172, div 0 [mask not inlined] | br 934 (reg 9), sel 12760, div 0 [mask not inlined] | br 1046 (reg 114), sel 9012, div 0 [mask not inlined] | br 975 (reg 146), sel 12555, div 0 [mask not inlined] |
| `selunpred` | br 951 (reg 63), sel 9179, div 0 TRACE DIFF p248,p324,p376 | br 841 (reg 80), sel 12843, div 0 TRACE DIFF p248,p324,p376,p500 | build | build |
| `subtle-bb` | br 1032 (reg 42), sel 9212, div 0 | br 934 (reg 9), sel 12718, div 0 | br 1046 (reg 114), sel 9052, div 0 | br 975 (reg 146), sel 12571, div 0 |
| `subtle-vol` | br 1032 (reg 42), sel 9182, div 0 | br 934 (reg 9), sel 12760, div 0 | br 1046 (reg 114), sel 9022, div 0 | br 975 (reg 146), sel 12555, div 0 |
| `volatile` | br 1032 (reg 41), sel 9150, div 0 | br 934 (reg 10), sel 12710, div 0 | br 909 (reg 113), sel 8976, div 0 | br 855 (reg 145), sel 12581, div 0 |
| `cmov` | br 1032 (reg 42), sel 9318, div 0 | br 934 (reg 9), sel 12898, div 0 | br 1046 (reg 114), sel 9158, div 0 | br 975 (reg 146), sel 12693, div 0 |
| `ctutils` | br 1032 (reg 42), sel 9382, div 0 | br 934 (reg 9), sel 12868, div 0 | build | build |

## dudect detail (x86-64, pinned core)

| set | candidate / toolchain / config | n | t raw | max t cropped | t 2nd order | max abs t | r(cycles, bits N) | median cycles fixed / random |
|---|---|---|---|---|---|---|---|---|
| p248 | asm_1.97.1_c1 | 40000 | -0.25 | 1.53 | -0.29 | 1.53 | 0.001 | 1880418 / 1880422 |
| p248 | bbmask_1.97.1_c1 | 40000 | 0.73 | -1.59 | 0.98 | 1.59 | 0.007 | 1824760 / 1824836 |
| p248 | bbuse_1.97.1_c1 | 40000 | 0.99 | 1.62 | 1.00 | 1.62 | 0.004 | 1996806 / 1998572 |
| p248 | cmov_1.97.1_c1 | 40000 | -1.80 | -1.75 | -1.71 | 1.80 | -0.015 | 2066186 / 2065292 |
| p248 | crate_1.97.1_c1 | 40000 | 0.95 | -2.18 | 0.81 | 2.18 | 0.007 | 1880696 / 1880736 |
| p248 | ctutils_1.97.1_c1 | 40000 | -2.08 | 2.54 | -2.04 | 2.54 | -0.006 | 1933558 / 1933374 |
| p248 | noinline_1.97.1_c1 | 40000 | 0.56 | 2.31 | -0.38 | 2.31 | -0.001 | 2170342 / 2170308 |
| p248 | none_1.97.1_c1 | 40000 | -1.68 | -32.36 | -0.68 | 32.36 | -0.007 | 1723208 / 1725604 |
| p248 | selunpred_1.97.1_c1 | 40000 | 0.86 | -30.00 | 0.99 | 30.00 | -0.011 | 1689784 / 1689996 |
| p248 | subtle-bb_1.97.1_c1 | 40000 | 1.25 | 1.85 | 1.50 | 1.85 | 0.006 | 1919848 / 1918636 |
| p248 | subtle-vol_1.97.1_c1 | 40000 | -0.64 | 2.08 | -1.10 | 2.08 | -0.007 | 2097646 / 2099070 |
| p248 | volatile_1.97.1_c1 | 40000 | -1.71 | 2.44 | -1.23 | 2.44 | 0.001 | 2019576 / 2018948 |
| p248 | weak_1.97.1_c1 | 40000 | -1.12 | 9.11 | -1.03 | 9.11 | 0.011 | 1901744 / 1902048 |
| p500 | asm_1.97.1_c1 | 20000 | -1.00 | 2.07 | -1.19 | 2.07 | -0.007 | 9190364 / 9181276 |
| p500 | bbmask_1.97.1_c1 | 20000 | 0.83 | 1.84 | 0.76 | 1.84 | 0.010 | 9529850 / 9522444 |
| p500 | bbuse_1.97.1_c1 | 20000 | -0.20 | 2.05 | -0.84 | 2.05 | 0.001 | 11077626 / 11071320 |
| p500 | cmov_1.97.1_c1 | 20000 | 0.56 | 2.03 | 0.45 | 2.03 | 0.004 | 9645654 / 9644220 |
| p500 | crate_1.97.1_c1 | 20000 | 1.54 | -1.79 | 1.80 | 1.80 | 0.003 | 9000824 / 9006222 |
| p500 | ctutils_1.97.1_c1 | 20000 | -1.05 | -1.57 | -1.00 | 1.57 | -0.007 | 10125990 / 10118992 |
| p500 | noinline_1.97.1_c1 | 20000 | -0.45 | -2.12 | -0.74 | 2.12 | -0.012 | 10119422 / 10124784 |
| p500 | none_1.97.1_c1 | 20000 | -0.35 | -2.10 | -0.08 | 2.10 | -0.014 | 9273312 / 9275278 |
| p500 | selunpred_1.97.1_c1 | 20000 | 0.99 | 2.79 | 0.97 | 2.79 | -0.000 | 9050510 / 9042016 |
| p500 | subtle-bb_1.97.1_c1 | 20000 | -1.13 | 2.94 | -0.43 | 2.94 | -0.004 | 10076254 / 10097576 |
| p500 | subtle-vol_1.97.1_c1 | 20000 | -1.02 | 1.70 | -1.00 | 1.70 | 0.016 | 9982910 / 9979952 |
| p500 | volatile_1.97.1_c1 | 20000 | 1.26 | 1.72 | 0.94 | 1.72 | -0.006 | 9422902 / 9425584 |
| p500 | weak_1.97.1_c1 | 20000 | 0.08 | -1.83 | 0.47 | 1.83 | -0.005 | 9490008 / 9496074 |

## Memcheck detail (x86-64)

| run | sets | errors in CT code | errors outside glibc allocator |
|---|---|---|---|
| 1.85.1_c1_asm | 3 | 0 branch + 0 other (0 simd-shift) | 0 |
| 1.85.1_c1_bbmask | 3 | 0 branch + 0 other (6 simd-shift) | 6 |
| 1.85.1_c1_bbuse | 3 | 0 branch + 0 other (0 simd-shift) | 0 |
| 1.85.1_c1_cmov | 3 | 0 branch + 0 other (0 simd-shift) | 0 |
| 1.85.1_c1_crate | 3 | 0 branch + 0 other (0 simd-shift) | 0 |
| 1.85.1_c1_ctutils | - | build failed | - |
| 1.85.1_c1_noinline | 3 | 0 branch + 0 other (0 simd-shift) | 0 |
| 1.85.1_c1_none | 3 | 55 branch + 0 other (0 simd-shift) | 55 |
| 1.85.1_c1_selunpred | - | build failed | - |
| 1.85.1_c1_subtle-bb | 3 | 0 branch + 0 other (6 simd-shift) | 6 |
| 1.85.1_c1_subtle-vol | 3 | 0 branch + 0 other (0 simd-shift) | 0 |
| 1.85.1_c1_volatile | 3 | 0 branch + 0 other (0 simd-shift) | 0 |
| 1.85.1_c1_weak | 3 | 14 branch + 0 other (0 simd-shift) | 14 |
| 1.97.1_c1_asm | 5 | 0 branch + 0 other (25 simd-shift) | 25 |
| 1.97.1_c1_asm_control | 1 | 0 branch + 0 other (0 simd-shift) | 1 |
| 1.97.1_c1_bbmask | 5 | 0 branch + 0 other (24 simd-shift) | 24 |
| 1.97.1_c1_bbuse | 5 | 0 branch + 0 other (22 simd-shift) | 22 |
| 1.97.1_c1_cmov | 5 | 0 branch + 0 other (21 simd-shift) | 21 |
| 1.97.1_c1_crate | 5 | 0 branch + 0 other (25 simd-shift) | 25 |
| 1.97.1_c1_ctutils | 5 | 0 branch + 0 other (23 simd-shift) | 23 |
| 1.97.1_c1_noinline | 5 | 0 branch + 0 other (22 simd-shift) | 22 |
| 1.97.1_c1_none | 5 | 81 branch + 0 other (26 simd-shift) | 107 |
| 1.97.1_c1_selunpred | 5 | 87 branch + 0 other (26 simd-shift) | 113 |
| 1.97.1_c1_subtle-bb | 5 | 0 branch + 0 other (22 simd-shift) | 22 |
| 1.97.1_c1_subtle-vol | 5 | 0 branch + 0 other (18 simd-shift) | 18 |
| 1.97.1_c1_volatile | 5 | 0 branch + 0 other (28 simd-shift) | 28 |
| 1.97.1_c1_weak | 5 | 20 branch + 0 other (28 simd-shift) | 48 |
