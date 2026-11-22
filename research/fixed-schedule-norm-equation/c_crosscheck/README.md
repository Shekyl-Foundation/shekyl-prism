# Cross-check against the C reference

The reference (mariascrs/PRISM_v2 at 87fad7d) was built unmodified except
for `qlapoti_stats.patch`, which counts passes, residue-passing passes and
passes whose M4 is positive, and prints one `QLSTAT` line per call:

    QLSTAT <passes> <found> <transformed> <mod_ok> <m4_positive>

Build (Release, system GMP from the Sage environment, cmake + ninja from pip):

    CC=gcc cmake -G Ninja -DPRISM_BUILD_TYPE=ref -DCMAKE_BUILD_TYPE=Release ..
    ninja prism_id2iso_benchmark_qlapoti_normeq_lvl{1,3,5}
    ./src/id2iso/ref/lvlN/test/prism_id2iso_benchmark_qlapoti_normeq_lvlN --iterations=400 --tests

Inputs are the benchmark's own: random O0-ideals of a random prime norm of
2·bits(p) bits (the norm equation first replaces the input by its shortest
equivalent, so the input's norm does not reach the search).

The benchmark's own `--tests` verification reports failure at this pin
with or without the patch (checked on an unpatched build, lvl1, 50
iterations); every call returned found = 1. Not investigated further here.
