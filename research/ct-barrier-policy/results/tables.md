## Recommendation table

asm-clean: control-flow trace identical across fixed, random and adversarial inputs under qemu (and correct outputs), no integer division; counted over the 4 build configs. Taint: memcheck errors in the constant-time code, 5 sets, x86-64, config c1. dudect: max |t| at P248 / P500, x86-64, 1.97.1, c1 (threshold 4.5).

| candidate | unsafe where | toolchain | x86-64 asm-clean | aarch64 asm-clean | x86-64 taint | x86-64 dudect max\|t\| (P248 / P500) |
|---|---|---|---|---|---|---|
| `asm` | unsafe asm in the signer crate | 1.97.1 | pending | 1/1 clean | pending | - |
| `asm` | unsafe asm in the signer crate | 1.85.1 | 0/3 clean (branch: c1,c2,c3) | 0/1 clean (branch: c1) | pending | not run |
| `crate` | unsafe asm, isolated in prism-ct-barrier | 1.97.1 | 1/1 clean | 1/1 clean | pending | - |
| `crate` | unsafe asm, isolated in prism-ct-barrier | 1.85.1 | 0/2 clean (branch: c1,c2) | 0/1 clean (branch: c1) | pending | not run |
| `none` | safe (negative control) | 1.97.1 | 0/1 clean (branch: c1) | pending | pending | - |
| `none` | safe (negative control) | 1.85.1 | 0/3 clean (branch: c1,c2,c3) | pending | pending | not run |
| `weak` | safe (negative control) | 1.97.1 | 0/1 clean (branch: c1) | pending | pending | - |
| `weak` | safe (negative control) | 1.85.1 | 0/2 clean (branch: c1,c2) | pending | pending | not run |
| `bbmask` | safe | 1.97.1 | 1/1 clean | 1/1 clean | pending | - |
| `bbmask` | safe | 1.85.1 | 0/2 clean (branch: c1,c2) | 0/1 clean (branch: c1) | pending | not run |
| `bbuse` | safe | 1.97.1 | 1/1 clean | 1/1 clean | pending | - |
| `bbuse` | safe | 1.85.1 | 0/2 clean (branch: c1,c2) | 0/1 clean (branch: c1) | pending | not run |
| `noinline` | safe | 1.97.1 | 1/1 clean | pending | pending | - |
| `noinline` | safe | 1.85.1 | 0/2 clean (branch: c1,c2) | pending | pending | not run |
| `selunpred` | safe (Rust >= 1.88) | 1.97.1 | pending | pending | pending | - |
| `selunpred` | safe (Rust >= 1.88) | 1.85.1 | pending | pending | pending | not run |
| `subtle-bb` | safe (subtle, core_hint_black_box) | 1.97.1 | 1/1 clean | pending | pending | - |
| `subtle-bb` | safe (subtle, core_hint_black_box) | 1.85.1 | 0/2 clean (branch: c1,c2) | pending | pending | not run |
| `subtle-vol` | unsafe read_volatile inside subtle | 1.97.1 | 1/1 clean | pending | pending | - |
| `subtle-vol` | unsafe read_volatile inside subtle | 1.85.1 | 0/2 clean (branch: c1,c2) | pending | pending | not run |
| `volatile` | unsafe read_volatile in the signer crate | 1.97.1 | 1/1 clean | pending | pending | - |
| `volatile` | unsafe read_volatile in the signer crate | 1.85.1 | 0/2 clean (branch: c1,c2) | pending | pending | not run |
| `cmov` | unsafe asm inside cmov | 1.97.1 | 1/1 clean | 1/1 clean | pending | - |
| `cmov` | unsafe asm inside cmov | 1.85.1 | 0/2 clean (branch: c1,c2) | 0/1 clean (branch: c1) | pending | not run |
| `ctutils` | unsafe asm inside cmov (via ctutils) | 1.97.1 | 1/1 clean | 1/1 clean | pending | - |
| `ctutils` | unsafe asm inside cmov (via ctutils) | 1.85.1 | n/a (does not build) | 0/0 clean | pending | not run |

## Per configuration (trace verdict: ok / BR = secret-dependent control flow / build = does not compile)

- `c1`: opt 3, fat LTO, cgu 1
- `c2`: opt 2, fat LTO, cgu 1
- `c3`: opt 3, no LTO, cgu 16
- `c4`: opt 3, fat LTO, cgu 1, target-cpu (x86: native = this Xeon, AVX-512; aarch64: neoverse-v1)

| candidate | 1.97.1 x86 c1 | 1.97.1 x86 c2 | 1.97.1 x86 c3 | 1.97.1 x86 c4 | 1.97.1 aar c1 | 1.97.1 aar c2 | 1.97.1 aar c3 | 1.97.1 aar c4 | 1.85.1 x86 c1 | 1.85.1 x86 c2 | 1.85.1 x86 c3 | 1.85.1 x86 c4 | 1.85.1 aar c1 | 1.85.1 aar c2 | 1.85.1 aar c3 | 1.85.1 aar c4 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `asm` | - | - | - | - | ok | - | - | - | BR | BR | BR | - | BR | - | - | - |
| `crate` | ok | - | - | - | ok | - | - | - | BR | BR | - | - | BR | - | - | - |
| `none` | BR | - | - | - | - | - | - | - | BR | BR | BR | - | - | - | - | - |
| `weak` | BR | - | - | - | - | - | - | - | BR | BR | - | - | - | - | - | - |
| `bbmask` | ok | - | - | - | ok | - | - | - | BR | BR | - | - | BR | - | - | - |
| `bbuse` | ok | - | - | - | ok | - | - | - | BR | BR | - | - | BR | - | - | - |
| `noinline` | ok | - | - | - | - | - | - | - | BR | BR | - | - | - | - | - | - |
| `selunpred` | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - |
| `subtle-bb` | ok | - | - | - | - | - | - | - | BR | BR | - | - | - | - | - | - |
| `subtle-vol` | ok | - | - | - | - | - | - | - | BR | BR | - | - | - | - | - | - |
| `volatile` | ok | - | - | - | - | - | - | - | BR | BR | - | - | - | - | - | - |
| `cmov` | ok | - | - | - | ok | - | - | - | BR | BR | - | - | BR | - | - | - |
| `ctutils` | ok | - | - | - | ok | - | - | - | build | build | build | build | build | - | - | - |

## Static shape (config c1, sum over the five sets of zi::reduce + ct::exact_round)

| candidate | 1.97.1 x86 | 1.97.1 aar | 1.85.1 x86 | 1.85.1 aar |
|---|---|---|---|---|
| `asm` | - | br 934 (reg 9), sel 12760, div 0 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p376,p500 |
| `crate` | br 1032 (reg 42), sel 9120, div 0 | br 934 (reg 9), sel 12760, div 0 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p376,p500 |
| `none` | br 949 (reg 58), sel 9168, div 0 TRACE DIFF p248,p324,p376 | - | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | - |
| `weak` | br 1048 (reg 42), sel 9261, div 0 TRACE DIFF p248,p324,p376 | - | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | - |
| `bbmask` | br 1032 (reg 41), sel 9150, div 0 | br 934 (reg 9), sel 12718, div 0 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p376,p500 |
| `bbuse` | br 1100 (reg 41), sel 9142, div 0 | br 974 (reg 10), sel 12710, div 0 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p376,p500 |
| `noinline` | br 1032 (reg 42), sel 9172, div 0 [mask not inlined] | - | br 0 (reg 0), sel 0, div 0 [mask not inlined] TRACE DIFF p248,p324,p376,p500,p664 | - |
| `selunpred` | - | - | - | - |
| `subtle-bb` | br 1032 (reg 42), sel 9212, div 0 | - | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | - |
| `subtle-vol` | br 1032 (reg 42), sel 9182, div 0 | - | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | - |
| `volatile` | br 1032 (reg 41), sel 9150, div 0 | - | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | - |
| `cmov` | br 1032 (reg 42), sel 9318, div 0 | br 934 (reg 9), sel 12898, div 0 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p324,p376,p500,p664 | br 0 (reg 0), sel 0, div 0 TRACE DIFF p248,p376,p500 |
| `ctutils` | br 1032 (reg 42), sel 9382, div 0 | br 934 (reg 9), sel 12868, div 0 | build | build |

## dudect detail (x86-64, pinned core)

| set | candidate / toolchain / config | n | t raw | max t cropped | t 2nd order | max abs t | r(cycles, bits N) | median cycles fixed / random |
|---|---|---|---|---|---|---|---|---|

## Memcheck detail (x86-64)

| run | sets | errors in CT code | errors outside glibc allocator |
|---|---|---|---|
