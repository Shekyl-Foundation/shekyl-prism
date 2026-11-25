#!/bin/bash
# Build the pinned reference (PRISM_v2 @ 87fad7d, + QLSTAT counting patch)
# at three values of QUAT_qlapoti_used_power_of_two: e-2 (pin), e-1, e.
# Asserts are kept (no NDEBUG) so the translation's own checks run.
set -e
export PATH=/tmp/cmv/bin:$PATH
for v in ${VARIANTS:-e2 e1 e0 e2np e0np}; do
  mkdir -p /workspace/qlr-c/bld-$v && cd /workspace/qlr-c/bld-$v
  CC=gcc cmake -G Ninja -DPRISM_BUILD_TYPE=ref -DCMAKE_BUILD_TYPE=Debug \
    -DCMAKE_C_FLAGS_DEBUG="-O2 -g" -DGMP_LIBRARY=SYSTEM \
    -DGMP=/workspace/mamba/envs/sage/lib/libgmp.so -DGMP_INCLUDE=/workspace/mamba/envs/sage/include \
    ../src-$v > cmake.log 2>&1
  ninja -j${JOBS:-3} prism_test_scheme_lvl1 prism_test_scheme_lvl3 prism_test_scheme_lvl5 > ninja.log 2>&1
  ls -la test/prism_test_scheme_lvl* 
done
