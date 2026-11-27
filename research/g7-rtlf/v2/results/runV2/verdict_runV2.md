# RULE-v2 application: runV2

### runV2: process validity

| process | seed | reference | perm | null arm flagged in | valid |
|---|---|---|---|---|---|
| v2_8192_s101 | 101 | f3 | 3-1-2-4-0 | none | yes |
| v2_8192_s102 | 102 | f1 | 1-4-2-3-0 | none | yes |
| v2_8192_s103 | 103 | f1 | 1-4-2-3-0 | none | yes |

### runV2: same-session injected-delay control (delay nominal, whole-run RTLF α=.01 / Welch t)

| process | phase | arm | n | mean shift ns | median shift ns | Welch t | RTLF .01 | flagged |
|---|---|---|---|---|---|---|---|---|
| v2_8192_s101 | adm | injL | 8192 | -59 | +223 | -0.16 | 1 | 1 |
| v2_8192_s102 | adm | injL | 8192 | +602 | +240 | +1.62 | 1 | 1 |
| v2_8192_s103 | adm | injL | 8192 | +245 | +222 | +0.47 | 1 | 1 |
| v2_8192_s101 | adm | injM | 8192 | -60 | +315 | -0.16 | 1 | 1 |
| v2_8192_s102 | adm | injM | 8192 | +1082 | +315 | +2.06 | 1 | 1 |
| v2_8192_s103 | adm | injM | 8192 | +696 | +315 | +1.08 | 1 | 1 |
| v2_8192_s101 | adm | injH | 8192 | +148 | +500 | +0.40 | 1 | 1 |
| v2_8192_s102 | adm | injH | 8192 | +430 | +500 | +8.53 | 1 | 1 |
| v2_8192_s103 | adm | injH | 8192 | +97 | +500 | +0.26 | 1 | 1 |
| v2_8192_s101 | herm | injL | 8192 | -589 | +222 | -1.26 | 1 | 1 |
| v2_8192_s102 | herm | injL | 8192 | -146 | +204 | -0.39 | 1 | 1 |
| v2_8192_s103 | herm | injL | 8192 | +16 | +204 | +0.02 | 1 | 1 |
| v2_8192_s101 | herm | injM | 8192 | -396 | +315 | -0.84 | 1 | 1 |
| v2_8192_s102 | herm | injM | 8192 | +760 | +315 | +1.18 | 1 | 1 |
| v2_8192_s103 | herm | injM | 8192 | -275 | +315 | -0.38 | 1 | 1 |
| v2_8192_s101 | herm | injH | 8192 | -41 | +518 | -0.08 | 1 | 1 |
| v2_8192_s102 | herm | injH | 8192 | +169 | +500 | +0.45 | 1 | 1 |
| v2_8192_s103 | herm | injH | 8192 | +53 | +500 | +0.07 | 1 | 1 |
| v2_8192_s101 | full | injL | 8192 | +4476 | +1649 | +1.58 | 1 | 1 |
| v2_8192_s102 | full | injL | 8192 | +2759 | +2000 | +0.83 | 1 | 1 |
| v2_8192_s103 | full | injL | 8192 | -412 | +1740 | -0.08 | 1 | 1 |
| v2_8192_s101 | full | injM | 8192 | +4292 | +3094 | +1.65 | 1 | 1 |
| v2_8192_s102 | full | injM | 8192 | +508 | +3409 | +0.17 | 1 | 1 |
| v2_8192_s103 | full | injM | 8192 | -225 | +3342 | -0.04 | 1 | 1 |
| v2_8192_s101 | full | injH | 8192 | +8384 | +6445 | +3.21 | 1 | 1 |
| v2_8192_s102 | full | injH | 8192 | +4352 | +5992 | +1.38 | 1 | 1 |
| v2_8192_s103 | full | injH | 8192 | +2268 | +6499 | +0.43 | 1 | 1 |

Smallest control level detected by RTLF in every valid process: adm: injL, herm: injL, full: injL

### runV2: control detection per chunk (n = 4096 rounds per chunk)

| phase | arm | chunks | RTLF .01 | RTLF .09 | Welch t > 4.5 |
|---|---|---|---|---|---|
| adm | null | 6 | 0 | 0 | 0 |
| adm | injL | 6 | 6 | 6 | 0 |
| adm | injM | 6 | 6 | 6 | 0 |
| adm | injH | 6 | 6 | 6 | 3 |
| herm | null | 6 | 0 | 1 | 0 |
| herm | injL | 6 | 6 | 6 | 0 |
| herm | injM | 6 | 6 | 6 | 0 |
| herm | injH | 6 | 6 | 6 | 1 |
| full | null | 6 | 0 | 0 | 0 |
| full | injL | 6 | 6 | 6 | 0 |
| full | injM | 6 | 6 | 6 | 0 |
| full | injH | 6 | 6 | 6 | 0 |

### runV2: fixture and key contrasts per process

| phase | contrast | s101 | s102 | s103 | processes flagged (same sign) | verdict |
|---|---|---|---|---|---|---|
| adm | f0 vs f1 | – | +1 ns t+0.6 | +0 ns t-0.9 | 0 (mixed/none) | no flag |
| adm | f0 vs f3 | +0 ns t+0.5 | – | – | 0 (mixed/none) | no flag |
| adm | f1 vs f3 | +0 ns t-0.3 | – | – | 0 (mixed/none) | no flag |
| adm | f2 vs f1 | – | +0 ns t+1.0 | +0 ns t-0.9 | 0 (mixed/none) | no flag |
| adm | f2 vs f3 | +0 ns t+0.0 | – | – | 0 (mixed/none) | no flag |
| adm | f3 vs f1 | – | +0 ns t-0.6 | +0 ns t-0.8 | 0 (mixed/none) | no flag |
| adm | f4 vs f1 | – | +0 ns t+1.5 | +0 ns t+1.1 | 0 (mixed/none) | no flag |
| adm | f4 vs f3 | +0 ns t-0.9 | – | – | 0 (mixed/none) | no flag |
| adm | krand vs kfix | -1 ns t-0.6 | +0 ns t-2.0 | +0 ns t+1.6 | 0 (mixed/none) | no flag |
| herm | f0 vs f1 | – | +0 ns t-1.0 | +0 ns t+0.3 | 0 (mixed/none) | no flag |
| herm | f0 vs f3 | +0 ns t-1.7 | – | – | 0 (mixed/none) | no flag |
| herm | f1 vs f3 | +0 ns t-0.0 | – | – | 0 (mixed/none) | no flag |
| herm | f2 vs f1 | – | +0 ns t+0.1 | +0 ns t+0.9 | 0 (mixed/none) | no flag |
| herm | f2 vs f3 | +0 ns t-1.7 | – | – | 0 (mixed/none) | no flag |
| herm | f3 vs f1 | – | +0 ns t-0.7 | +0 ns t-0.0 | 0 (mixed/none) | no flag |
| herm | f4 vs f1 | – | +0 ns t-1.1 | +0 ns t-1.1 | 0 (mixed/none) | no flag |
| herm | f4 vs f3 | +0 ns t-1.4 | – | – | 0 (mixed/none) | no flag |
| herm | krand vs kfix | +0 ns t+0.9 | +0 ns t+0.3 | +0 ns t+0.1 | 0 (mixed/none) | no flag |
| full | f0 vs f1 | – | +75 ns t-1.9 | -56 ns t-1.3 | 0 (mixed/none) | no flag |
| full | f0 vs f3 | -18 ns t+0.5 | – | – | 0 (mixed/none) | no flag |
| full | f1 vs f3 | -92 ns t-0.1 | – | – | 0 (mixed/none) | no flag |
| full | f2 vs f1 | – | +19 ns t-0.7 | +92 ns t-1.1 | 0 (mixed/none) | no flag |
| full | f2 vs f3 | -295 ns t+0.7 | – | – | 0 (mixed/none) | no flag |
| full | f3 vs f1 | – | -73 ns t-1.6 | -37 ns t-1.2 | 0 (mixed/none) | no flag |
| full | f4 vs f1 | – | +94 ns t+0.2 | -148 ns t-0.7 | 0 (mixed/none) | no flag |
| full | f4 vs f3 | -129 ns t+0.3 | – | – | 0 (mixed/none) | no flag |
| full | krand vs kfix | -19 ns t-0.8 | -75 ns t-0.5 | -56 ns t+0.8 | 0 (mixed/none) | no flag |

### runV2: identity contrasts (fixture vs pooled other fixtures); shift = median diff, R = RTLF .01 flag (>=3 ticks), T = |Welch t|>4.5

| phase | fixture | s101 | s102 | s103 | flagged procs | same sign |
|---|---|---|---|---|---|---|
| adm | f0 | +0 ns (mean +453) t+0.8 R0 | +1 ns (mean -45) t-0.6 R0 | +0 ns (mean -361) t-1.9 R0 | 0 | no |
| adm | f1 | +0 ns (mean -144) t-0.5 R0 | +0 ns (mean -86) t-1.1 R0 | +0 ns (mean +43) t+0.1 R0 | 0 | no |
| adm | f2 | +0 ns (mean +46) t+0.1 R0 | +0 ns (mean +235) t+0.9 R0 | +0 ns (mean -370) t-1.9 R0 | 0 | no |
| adm | f3 | +0 ns (mean +23) t+0.1 R0 | +0 ns (mean -124) t-1.7 R0 | +0 ns (mean -338) t-1.8 R0 | 0 | no |
| adm | f4 | +0 ns (mean -378) t-1.9 R0 | +0 ns (mean +19) t+0.2 R0 | +0 ns (mean +1026) t+1.6 R0 | 0 | no |
| herm | f0 | +0 ns (mean -442) t-2.4 R0 | +0 ns (mean -212) t-1.5 R0 | +0 ns (mean +186) t+0.3 R0 | 0 | no |
| herm | f1 | +0 ns (mean +547) t+1.0 R0 | +0 ns (mean +253) t+0.7 R0 | +0 ns (mean -103) t-0.1 R0 | 0 | no |
| herm | f2 | +0 ns (mean -414) t-2.3 R0 | +0 ns (mean +325) t+0.8 R0 | +0 ns (mean +1094) t+1.3 R0 | 0 | no |
| herm | f3 | +0 ns (mean +567) t+1.2 R0 | +0 ns (mean -80) t-0.6 R0 | +0 ns (mean -107) t-0.2 R0 | 0 | no |
| herm | f4 | +0 ns (mean -257) t-1.4 R0 | +0 ns (mean -286) t-2.0 R0 | +0 ns (mean -1070) t-2.5 R0 | 0 | no |
| full | f0 | +129 ns (mean +517) t+0.3 R0 | +74 ns (mean -3942) t-2.0 R0 | -38 ns (mean -2790) t-0.8 R0 | 0 | no |
| full | f1 | +19 ns (mean -1177) t-0.6 R0 | -20 ns (mean +2964) t+1.2 R0 | +37 ns (mean +5385) t+1.3 R0 | 0 | no |
| full | f2 | -240 ns (mean +1635) t+0.7 R0 | +0 ns (mean +323) t+0.1 R0 | +148 ns (mean -1328) t-0.3 R0 | 0 | no |
| full | f3 | +138 ns (mean -918) t-0.5 R0 | -111 ns (mean -2981) t-1.4 R0 | -18 ns (mean -2387) t-0.6 R0 | 0 | no |
| full | f4 | -18 ns (mean -58) t-0.0 R0 | +92 ns (mean +3636) t+1.4 R0 | -148 ns (mean +1120) t+0.3 R0 | 0 | no |
