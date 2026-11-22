## Cycles, primary: T_prov + 1 exact cleanup step (median, rdtsc, 2.4 GHz TSC)

| Set | CT Z[i] (steps) | exact LLL, GMP | exact LLL, num-bigint | exact Z[i], GMP (mean steps) | CT / LLL-GMP | CT / LLL-num | model mul ratio | CT / exact Z[i]-GMP |
|---|---|---|---|---|---|---|---|---|
| P248 | 2.03 M (301) | 8.72 M | 17.82 M | 0.43 M (162) | 0.233 | 0.114 | 0.480 | 4.77 |
| P324 | 3.35 M (387) | 16.71 M | 35.18 M | 0.65 M (210) | 0.201 | 0.095 | 0.470 | 5.16 |
| P376 | 4.70 M (446) | 25.00 M | 53.78 M | 0.85 M (243) | 0.188 | 0.087 | 0.470 | 5.52 |
| P500 | 10.40 M (587) | 49.63 M | 115.27 M | 1.44 M (322) | 0.209 | 0.090 | 0.470 | 7.24 |
| P664 | 20.15 M (773) | 101.24 M | 243.15 M | 2.59 M (426) | 0.199 | 0.083 | 0.470 | 7.78 |

## Cycles, secondary: empirical T, no cleanup (unsafe on Rolletschek inputs) (median, rdtsc, 2.4 GHz TSC)

| Set | CT Z[i] (steps) | exact LLL, GMP | exact LLL, num-bigint | exact Z[i], GMP (mean steps) | CT / LLL-GMP | CT / LLL-num | model mul ratio | CT / exact Z[i]-GMP |
|---|---|---|---|---|---|---|---|---|
| P248 | 1.16 M (189) | 8.86 M | 18.07 M | 0.43 M (162) | 0.131 | 0.064 | 0.299 | 2.74 |
| P324 | 1.83 M (239) | 17.12 M | 35.87 M | 0.65 M (210) | 0.107 | 0.051 | 0.291 | 2.80 |
| P376 | 2.95 M (283) | 27.31 M | 58.39 M | 0.93 M (243) | 0.108 | 0.051 | 0.298 | 3.18 |
| P500 | 6.15 M (359) | 51.95 M | 120.51 M | 1.50 M (322) | 0.118 | 0.051 | 0.286 | 4.10 |
| P664 | 11.62 M (465) | 101.83 M | 243.93 M | 2.60 M (426) | 0.114 | 0.048 | 0.279 | 4.47 |


## dudect (fixed vs random input), threshold |t| = 4.5

| Set | impl | measurements | t raw | max t cropped | t 2nd order | max abs t | r(cycles, bits N) random class | verdict |
|---|---|---|---|---|---|---|---|---|
| P248 | ct | 60,000 | -0.76 | -2.20 | -0.92 | 2.20 | -0.005 | no leak detected |
| P248 | ct_emp | 100,000 | -0.43 | 1.73 | -1.38 | 1.73 | 0.005 | no leak detected |
| P248 | gauss | 20,000 | -44.34 | -61.89 | -4.10 | 61.89 | 0.307 | leak |
| P248 | lll_gmp | 20,000 | 16.17 | 88.79 | 0.14 | 88.79 | 0.254 | leak |
| P324 | ct | 60,000 | -0.25 | -2.63 | -0.23 | 2.63 | -0.008 | no leak detected |
| P324 | ct_emp | 100,000 | 0.25 | 2.13 | 0.48 | 2.13 | 0.000 | no leak detected |
| P324 | gauss | 20,000 | 4.24 | 65.96 | 0.93 | 65.96 | 0.100 | leak |
| P376 | ct | 40,000 | -1.12 | -1.92 | -1.54 | 1.92 | -0.000 | no leak detected |
| P376 | ct_emp | 60,000 | 0.54 | 1.25 | -0.44 | 1.25 | 0.012 | no leak detected |
| P376 | gauss | 20,000 | -4.74 | -46.99 | 0.64 | 46.99 | 0.065 | leak |
| P500 | ct | 30,000 | 1.20 | 2.93 | 1.16 | 2.93 | -0.002 | no leak detected |
| P500 | ct_emp | 50,000 | 1.02 | 1.73 | 0.71 | 1.73 | 0.000 | no leak detected |
| P500 | gauss | 20,000 | 7.34 | 154.17 | -0.87 | 154.17 | 0.040 | leak |
| P664 | ct | 30,000 | 1.04 | 1.88 | 0.63 | 1.88 | -0.002 | no leak detected |
| P664 | ct_emp | 40,000 | -1.40 | -3.61 | 0.46 | 3.61 | 0.005 | no leak detected |
| P664 | ct_emp_rep2 | 40,000 | 1.29 | 2.21 | 0.35 | 2.21 | -0.002 | no leak detected |
| P664 | gauss | 20,000 | 2.87 | 50.46 | -1.92 | 50.46 | 0.142 | leak |
