#!/bin/sh
# Copyright is not required for a shell script. This driver is the
# reproduction of the salted verifier corpus, under the approved brief.
#
# Usage: PRISM_REF_BUILD=/path/to/cmake-build scripts/export_s5_vectors.sh
# The build is the pinned tree configured with -DPRISM_BUILD_TYPE=ref
# -DGMP_LIBRARY=MINI -DENABLE_SIGN=ON, and it must already contain the
# per-level static libraries. Secret-key scratch lives in a private
# temporary directory and is removed when the script exits.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
build=${PRISM_REF_BUILD:?set PRISM_REF_BUILD to the reference cmake build}
src=${PRISM_REF_SOURCE:-$root/external/PRISM_v2}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# The pinned ideal-to-isogeny aborts on some seeds (mini-gmp divides by
# zero). The seeds below are the ones that produced the committed vectors.
# Each number is the byte repeated 48 times. Scratch files live in a
# private directory from mktemp, removed on every exit.

# The pinned Cornacchia loop divides by zero when its cofactor is not
# positive. The patch rejects that cofactor before the loop. It is
# applied to a copy of the quaternion archive for this exporter only;
# the submodule and the reference build stay as pinned.
patched_quaternion() {
    dest=$work/libprism_quaternion_generic.a
    if [ -f "$dest" ]; then
        printf '%s\n' "$dest"
        return
    fi
    cp "$build/src/quaternion/ref/generic/libprism_quaternion_generic.a" "$dest"
    cp "$src/src/quaternion/ref/generic/integers.c" "$work/integers.c"
    patch -d "$work" -p7 --forward --input="$root/scripts/c/cornacchia-positive.patch" >&2
    /usr/bin/cc -O3 -std=gnu11 -DNDEBUG -fPIC -fvisibility=hidden -flto=auto -fno-fat-lto-objects \
        -DENABLE_SIGN -DGMP_LIMB_BITS=64 -DHAVE_UINT128 -DMINI_GMP \
        -DPRISM_BUILD_TYPE_REF -DRADIX_64 -DTARGET_AMD64 -DTARGET_OS_UNIX \
        -I"$src/include" -I"$src/src/common/generic/include" \
        -I"$src/src/quaternion/ref/generic/include" \
        -I"$src/src/quaternion/ref/generic/internal_quaternion_headers" \
        -I"$src/src/mini-gmp" \
        -c -o "$work/integers.c.o" "$work/integers.c"
    ar r "$dest" "$work/integers.c.o" >&2
    printf '%s\n' "$dest"
}

compile() {
    variant=$1
    quat=$(patched_quaternion)
    /usr/bin/cc -O2 -std=gnu11 -DNDEBUG ${rngflag:-} \
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
        -o "$build/export_s5_$variant" "$root/scripts/c/export_s5_vectors.c" \
        -Wl,--start-group \
        "$build/src/signature/ref/$variant/libprism_signature_$variant.a" \
        "$build/src/id2iso/ref/$variant/libprism_id2iso_$variant.a" \
        "$build/src/verification/ref/$variant/libprism_verification_$variant.a" \
        "$quat" \
        "$build/src/hd/ref/$variant/libprism_hd_$variant.a" \
        "$build/src/ec/ref/$variant/libprism_ec_$variant.a" \
        "$build/src/gf/ref/$variant/libprism_gf_$variant.a" \
        "$build/src/mp/ref/generic/libprism_mp_generic.a" \
        "$build/src/precomp/ref/$variant/libprism_precomp_$variant.a" \
        "$build/libGMP.a" \
        "$build/src/common/generic/libprism_common_sys.a" \
        -Wl,--end-group -lm
}

# Regeneration is authorized by the dated salt brief, never an implicit
# overwrite invoked just to silence a changed vector.
case "${1:-}" in
    backend) rngflag=-DPRISM_SECURE_RNG ;;
    fixture-backend) rngflag= ;;
    regenerate)
        [ "${2:-}" = "salt-brief-2026-11-12" ] || { echo "name the approved regeneration decision" >&2; exit 2; }
        rngflag=
        ;;
    *) echo "usage: $0 backend | fixture-backend | regenerate salt-brief-2026-11-12" >&2; exit 2 ;;
esac
for variant in lvl1 lvl3 lvl5; do
    compile "$variant"
    if [ "$1" = backend ]; then
        mv "$build/export_s5_$variant" "$build/prism_signer_$variant"
    fi
done
if [ "$1" = regenerate ]; then
    env -u CARGO_TARGET_DIR cargo build --release --locked -p prism-compare --bin fixture_challenges -p prism-verify --bin verify_bytes --bin s5_challenge
    python3 "$root/scripts/regenerate_salted_vectors.py" --decision "$2" --backend-dir "$build"
fi
