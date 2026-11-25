#!/bin/sh
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Gives the integer crate's committed vectors to the C reference's ibz_*
# functions (scripts/c/int_conventions.c) and fails if any result differs.
#
# Usage: PRISM_REF_BUILD=/path/to/cmake-build scripts/check_int_conventions.sh
# The build is the one scripts/export_s5_vectors.sh takes: the pinned tree
# configured with -DPRISM_BUILD_TYPE=ref -DGMP_LIBRARY=MINI -DENABLE_SIGN=ON.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
build=${PRISM_REF_BUILD:?set PRISM_REF_BUILD to the reference cmake build}
src=${PRISM_REF_SOURCE:-$root/external/PRISM_v2}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
variant=lvl1

/usr/bin/cc -O2 -std=gnu11 \
    -DENABLE_SIGN -DGMP_LIMB_BITS=64 -DHAVE_UINT128 -DMINI_GMP \
    -DPRISM_BUILD_TYPE_REF -DPRISM_VARIANT="$variant" -DRADIX_64 \
    -DTARGET_AMD64 -DTARGET_OS_UNIX \
    -I"$src/include" -I"$src/src/common/generic/include" \
    -I"$src/src/quaternion/ref/generic/include" \
    -I"$src/src/quaternion/ref/generic/internal_quaternion_headers" \
    -I"$src/src/precomp/ref/$variant/include" \
    -I"$src/src/gf/ref/include" -I"$src/src/gf/ref/$variant/include" \
    -I"$src/src/ec/ref/include" -I"$src/src/hd/ref/include" \
    -I"$src/src/id2iso/ref/include" -I"$src/src/mini-gmp" \
    -o "$work/int_conventions" "$root/scripts/c/int_conventions.c" \
    -Wl,--start-group \
    "$build/src/id2iso/ref/$variant/libprism_id2iso_$variant.a" \
    "$build/src/quaternion/ref/generic/libprism_quaternion_generic.a" \
    "$build/src/hd/ref/$variant/libprism_hd_$variant.a" \
    "$build/src/ec/ref/$variant/libprism_ec_$variant.a" \
    "$build/src/gf/ref/$variant/libprism_gf_$variant.a" \
    "$build/src/mp/ref/generic/libprism_mp_generic.a" \
    "$build/src/precomp/ref/$variant/libprism_precomp_$variant.a" \
    "$build/libGMP.a" \
    "$build/src/common/generic/libprism_common_sys.a" \
    -Wl,--end-group -lm

"$work/int_conventions" "$root/prism-int/tests/data/vectors.tsv"
