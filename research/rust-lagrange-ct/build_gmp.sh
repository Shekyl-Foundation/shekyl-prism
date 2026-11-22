#!/bin/sh
# Build with the GMP baselines against a system libgmp (here the SageMath
# micromamba env's). GMP=<prefix> overrides.
GMP=${GMP:-/workspace/mamba/envs/sage}
export LIBRARY_PATH=$GMP/lib C_INCLUDE_PATH=$GMP/include CPATH=$GMP/include LD_LIBRARY_PATH=$GMP/lib
export RUSTFLAGS="-L $GMP/lib -C link-arg=-Wl,-rpath,$GMP/lib"
exec cargo build --release -j2 --features gmp --target-dir target/gmp "$@"
