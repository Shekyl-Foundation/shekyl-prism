# Oracle-only fixtures (expected values come from the oracle, not from Rust)

Do not confuse with `../../fixtures/` (merged at d5acb8e), whose expected values were produced by the Rust prototype and
then cross-checked against the oracle. Here the order was: generate inputs and expected values from the oracle alone
(commit `5d0cd57`, before the Rust harness was built or run), then confirm the prototype (`confirm_oracle_fixtures.py`).

Rows: `case<TAB>verdict<TAB>input<TAB>T<TAB>column HNF<TAB>note`.

* `input` = `n xn yn u0..u3 v0..v3 s xq yq`, hex, `-` sign. `n` the key norm `N`; `(xn, yn)` the source ideal's root, i.e. the
  ideal is `O0·N + O0·(xn + yn·i + j)` with `x` even, `y` odd in `[0, 2N)` (columns of `GenericHnf`); `(u, v)` a Gaussian
  basis of that ideal (oracle's own Lagrange reduction, possibly sheared or unit-multiplied); `s` the odd challenge scalar,
  `q = 2^(2e−5) − s²`; `(xq, yq)` the challenge root mod `q` (any representative).
* `verdict` is the oracle's own: `ok`, `reject-admission` (key basis refused), `challenge-refused` (the public degree check
  fails), `marked` (root or unit check fails: all nine outputs marked).
* `T` (ok rows) is the carried norm `N·q`, hex.
* `column HNF` (ok rows) is the canonical **column** Hermite normal form of the intersection `I_sec ∩ I_q` in **raw O0
  numerator units** (a vector `(a,b,c,d)` stands for the quaternion `(a + bi + cj + dk)/2`), 16 hex entries, row-major
  `M[0][0] M[0][1] M[0][2] M[0][3] M[1][0] ...`. Convention: the four **columns** of `M` are a Z-basis of the lattice; `M` is
  upper triangular (`M[i][j] = 0` for `i > j`); `M[i][i] > 0`; `0 ≤ M[i][j] < M[i][i]` for `j > i` (entries to the right of a
  pivot are reduced modulo it). This is PARI `mathnf` of the column-generator matrix, and the layout of the prototype's
  `GenericHnf` columns `(2N,0,0,0), (0,2N,0,0), (x,y,1,0), (2N−y,x,0,1)`; in these fixtures it is always
  `[[2T,0,x,2T−y],[0,2T,y,x],[0,0,1,0],[0,0,0,1]]` for the parity-lifted CRT root `(x,y)` of norm `T = N·q`.
* The HNF is computed from the **ideals alone**: `I_sec` and `I_q` by left closure of `{N, root}` and `{q, root_q}`, then
  intersected by four routes (Sage submodule intersection, PARI dual lattice, kernel of the residue map, `q·I_sec + N·I_q`),
  which must agree, and the result must equal the generic shape at the CRT root. The basis `(u, v)` and the
  specialized formula play no part in the expected value.
