#!/bin/sh
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Builds the pinned C reference with scripts/c/ibz-trace.patch applied to a
# copy, and our exporter (scripts/c/export_s5_vectors.c) against it, for
# scripts/measure_integer_widths.py. The submodule is not changed.
#
# Usage: scripts/build_traced_reference.sh <work-directory> [jobs]
# Leaves <work>/traced_lvl1, traced_lvl3 and traced_lvl5.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
work=${1:?name a work directory}
jobs=${2:-2}
[ ! -e "$work/src" ] || { echo "$work/src exists; name a new directory" >&2; exit 2; }
mkdir -p "$work"
work=$(CDPATH= cd -- "$work" && pwd)
cp -R "$root/external/PRISM_v2" "$work/src"
rm -f "$work/src/.git"
# Both patches name their files from the repository's root.
patch -d "$work/src" -p3 --forward --input="$root/scripts/c/cornacchia-positive.patch"
patch -d "$work/src" -p3 --forward --input="$root/scripts/c/ibz-trace.patch"
# -g so that a recorded return address resolves to a function, inlined
# callers included; no sibling-call optimization, so that an ibz_* call in
# tail position still returns into the function that made it.
# ENABLE_STRICT would turn the wrappers' missing prototypes into errors.
cmake -S "$work/src" -B "$work/build" -DCMAKE_BUILD_TYPE=Release -DPRISM_BUILD_TYPE=ref \
    -DGMP_LIBRARY=MINI -DENABLE_STRICT=OFF "-DCMAKE_C_FLAGS=-g -fno-optimize-sibling-calls"
cmake --build "$work/build" --parallel "$jobs" --target prism_lvl1 prism_lvl3 prism_lvl5
src=$work/src
build=$work/build
for variant in lvl1 lvl3 lvl5; do
    # The link line of scripts/export_s5_vectors.sh, with the fixture
    # generator (a seed decides every draw), debug information, and no PIE,
    # so that addresses are the same in every run.
    /usr/bin/cc -O2 -g -fno-optimize-sibling-calls -no-pie -std=gnu11 -DNDEBUG \
        -DENABLE_SIGN -DGMP_LIMB_BITS=64 -DHAVE_UINT128 -DMINI_GMP \
        -DPRISM_BUILD_TYPE_REF -DPRISM_VARIANT="$variant" -DRADIX_64 \
        -DTARGET_AMD64 -DTARGET_OS_UNIX \
        -I"$src/include" -I"$src/src/common/generic/include" \
        -I"$src/src/quaternion/ref/generic/include" \
        -I"$src/src/quaternion/ref/generic/internal_quaternion_headers" \
        -I"$src/src/precomp/ref/$variant/include" \
        -I"$src/src/gf/ref/include" -I"$src/src/gf/ref/$variant/include" \
        -I"$src/src/ec/ref/include" -I"$src/src/hd/ref/include" \
        -I"$src/src/id2iso/ref/include" \
        -I"$src/src/verification/ref/include" \
        -I"$src/src/signature/ref/include" -I"$src/src/mini-gmp" \
        -o "$work/traced_$variant" "$root/scripts/c/export_s5_vectors.c" \
        -Wl,--start-group \
        "$build/src/signature/ref/$variant/libprism_signature_$variant.a" \
        "$build/src/id2iso/ref/$variant/libprism_id2iso_$variant.a" \
        "$build/src/verification/ref/$variant/libprism_verification_$variant.a" \
        "$build/src/quaternion/ref/generic/libprism_quaternion_generic.a" \
        "$build/src/hd/ref/$variant/libprism_hd_$variant.a" \
        "$build/src/ec/ref/$variant/libprism_ec_$variant.a" \
        "$build/src/gf/ref/$variant/libprism_gf_$variant.a" \
        "$build/src/mp/ref/generic/libprism_mp_generic.a" \
        "$build/src/precomp/ref/$variant/libprism_precomp_$variant.a" \
        "$build/libGMP.a" \
        "$build/src/common/generic/libprism_common_sys.a" \
        -Wl,--end-group -lm
done
echo "built $work/traced_lvl1 $work/traced_lvl3 $work/traced_lvl5"
