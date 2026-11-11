# Sage accept-path model report

Pin: `external/Theta_dim4` @ `4770b24a61e6aa15da4cac1b9d82fd7b9a05f459`.

## Confirmed (all six §5 vectors)

- Ran: Weil `e(R1,R2)=e(P1,P2)^q` after recovering the signature's second
  basis point as the C difference (the exporter swaps Q/PmQ into VkQ=R).
- Derived: product basis symplectic after applying `[s]`.
- Derived: `Φ̂Φ = 2^{2e′−1} I` over `Z/2^e`.
- Derived: `K″1` and `K″2` isotropic (geometric Weil on the product).
- Derived: symplectic completions `M1_0`, `M2`, `M1` exist.

## Refuted on `s5_lvl1.txt` (honest NIST-I)

Step 5 / Codomain:

```
C1.zero() != C2.hadamard().zero()
```

after `IsogenyChainDim2` of lengths `e′=246` and `e′−1=245` with kernels
`K″1`, `[2]⟨s b1+b2, s b3+q b4⟩` and theta base-change matrices
`N_i = base_change_theta_dim2(M0·M_i mod 4, e4)`.

Stopped per brief §4: do not patch around a refuted Derived claim; do not
port accept Rust. Construction details that may still be wrong (and would
make this a model bug rather than a claim refutation) are recorded in
`scripts/sage/s5_model.sage`: M0 composition order, kernel orientation,
and whether the dim-2 reduction of `starting_two_symplectic_matrices`
matches Dartois's matrices exactly.
