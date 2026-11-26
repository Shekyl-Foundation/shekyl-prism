# Tail proof and P500/P664 search: working notes (in progress)

- Milestone 1: `cellmap.c` proves the tail: fewer than 5 further swaps from every post-swap state with λ ∈ [2⁻¹¹, 1]. It uses adaptive cell mapping over G₂…G₅, takes 5 s and uses no inherited facts. See `results/cellmap_tail5_from_2^-11.txt`.
- Lemma C is load-bearing (see `results/cellmap_ablation.txt`). It is re-derived in `tail_lemmas.sage`.
