# Worst-Case Step Bound for Fixed-Schedule $\mathbb{Z}[i]$-Lagrange Reduction

Research for Shekyl PRISM. Step-counting and analytical bound model; no
constant-time or security claim.

PRISM is the signature scheme of ePrint 2026/443 (Andrea Basso, Giacomo Borin,
Wouter Castryck, Maria Corte-Real Santos, Riccardo Invernizzi, Antonin Leroux,
Luciano Maino, Frederik Vercauteren, and Benjamin Wesolowski). `shekyl-prism`
implements it.

## Question

[`research/fixed-schedule-reduction/`](../fixed-schedule-reduction/) calibrated
fixed-schedule $\mathbb{Z}[i]$-Lagrange step counts $T_{\text{emp}} = 189, 239,
283, 359, 465$ for P248, P324, P376, P500, and P664 from empirical maxima over
sampled keys and messages (with secret norm bound $B = \lceil L/2 \rceil + 32$).

Can we replace that empirical maximum with a **provable worst-case upper bound**
$T_{\text{prov}}$ on the number of steps of the exact $\mathbb{Z}[i]$-Lagrange
algorithm (`zi_gauss_fixed`) as a function of the public parameter set $(p, q, B)$,
and compare:

1. **Theoretical bound vs. observed distribution:** Check classical Gauss/Lagrange
   reduction analyses (geometric norm decrease per step in $\mathbb{Z}$ and
   $\mathbb{Z}[i]$, e.g., Gauss 1801, Lagrange 1773, Vallée 1991, Semaev 2001,
   Yap 1992, Napias 1996, Nguyen–Stehlé 2004/2009) against the exact rounding
   rule `(2*h + nu) // (2*nu)` used in `zi_gauss_fixed`.
2. **Adversarial & worst-case bases:** Construct explicit worst-case inputs near
   the bound (e.g., $\mathbb{Z}[i]$-analogues of Fibonacci/continued-fraction chains
   in $\mathbb{Z}[i]$ with norm quotient $\approx 1 + \sqrt{2}$ or minimal step
   shrinkage) and measure whether any legal ideal basis exceeds $T_{\text{emp}}$ or
   approaches $T_{\text{prov}}$.
3. **Bounded-precision quotients & batching:** Check whether bounded-precision
   quotients (e.g., top $P \in \{16, 32, 64\}$ bits) or a half-GCD-style batching
   preserve a provable convergence guarantee with fewer or cheaper steps.
4. **Proposed schedule & margin:** Propose per-set fixed step counts $T$ with
   proven margin over every valid input, and quantify the cost change versus
   $T_{\text{emp}}$ and exact LLL.

## Plan

1. Verify exact step-by-step algebraic bounds for `zi_gauss_fixed` (including
   tie-breaking in `(2*h + nu) // (2*nu)`, boundary norm ratios $\operatorname{nrd}(v)/\operatorname{nrd}(u)$,
   one-step vs. two-step geometric decay, and the determinant / $\lambda_1$ lower
   bound for ideal lattices of norm $T = N \cdot q$).
2. Implement adversarial generators and high-sample verification across
   P248, P324, P376, P500, and P664.
3. Analyze bounded-precision quotients (`prec = 8, 16, 32, 64`) for provable
   error bounds on $\mu$ and step-count overhead, plus half-GCD / Lehmer-style
   batching feasibility.
4. Write `REPORT.md`, commit results and plots, and fold into `research`.
