## Cycles (median, rdtsc at 2.4 GHz TSC)

| Set | CT Z[i] (T steps) | exact LLL, GMP | exact LLL, num-bigint | exact Z[i], GMP (mean steps) | CT / LLL-GMP | CT / LLL-num | model step ratio | CT / exact Z[i]-GMP |
|---|---|---|---|---|---|---|---|---|
| P248 | 1.16 M (189) | 8.86 M | 18.07 M | 0.43 M (162) | 0.131 | 0.064 | 0.299 | 2.74 |
| P324 | 1.83 M (239) | 17.12 M | 35.87 M | 0.65 M (210) | 0.107 | 0.051 | 0.291 | 2.80 |
| P376 | 2.95 M (283) | 27.31 M | 58.39 M | 0.93 M (243) | 0.108 | 0.051 | 0.298 | 3.18 |
| P500 | 6.15 M (359) | 51.95 M | 120.51 M | 1.50 M (322) | 0.118 | 0.051 | 0.286 | 4.10 |
| P664 | 11.62 M (465) | 101.83 M | 243.93 M | 2.60 M (426) | 0.114 | 0.048 | 0.279 | 4.47 |

## dudect (fixed vs random input), threshold |t| = 4.5

| Set | impl | measurements | t raw | max t cropped | t 2nd order | max abs t | r(cycles, bits N) random class | verdict |
|---|---|---|---|---|---|---|---|---|
| P248 | ct | 100,000 | -0.43 | 1.73 | -1.38 | 1.73 | 0.005 | no leak detected |
| P248 | gauss | 20,000 | -44.34 | -61.89 | -4.10 | 61.89 | 0.307 | leak |
