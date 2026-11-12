#!/bin/sh
# Copyright is not required for a shell script. This driver is the
# reproduction of prism-verify/testdata/s5_lvl{1,3,5}.txt.
#
# Usage: PRISM_REF_BUILD=/path/to/cmake-build scripts/export_s5_vectors.sh
# The build is the pinned tree configured with -DPRISM_BUILD_TYPE=ref
# -DGMP_LIBRARY=MINI -DENABLE_SIGN=ON, and it must already contain the
# per-level static libraries. Secret-key scratch lives in a private
# temporary directory and is removed when the script exits.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
build=${PRISM_REF_BUILD:?set PRISM_REF_BUILD to the reference cmake build}
src=$root/external/PRISM_v2
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
    /usr/bin/cc -O2 -std=gnu11 -DNDEBUG \
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

one() {
    n=$1
    name=$2
    key_seed=$3
    sign_seed=$4
    out=${5:-s5_lvl$n.txt}
    variant=lvl$n
    compile "$variant"
    scratch=$work/$variant-$out-key
    python3 -c "import sys; sys.stdout.buffer.write(bytes([$key_seed]) * 48)" | "$build/export_s5_$variant" keygen "$scratch" > "$scratch.pub"
    coefficient=$(awk '{print $2}' "$scratch.pub")
    pair=$(cargo run --quiet --manifest-path "$root/Cargo.toml" --bin s5_vector -- index "$name" "$coefficient")
    scalar=$(printf '%s\n' "$pair" | sed -n '2p')
    python3 -c "import sys; sys.stdout.buffer.write(bytes([$sign_seed]) * 48)" | "$build/export_s5_$variant" sign "$scratch" "$scalar" > "$scratch.sig"
    cat "$scratch.pub" "$scratch.sig" | cargo run --quiet --manifest-path "$root/Cargo.toml" --bin s5_vector -- commit "$name" "$root/prism-verify/testdata/$out"
    printf 'keygen_seed %s\nsign_seed %s\n' "$key_seed" "$sign_seed" >> "$root/prism-verify/testdata/$out"
}

positive() {
    n=$1
    name=$2
    count=$3
    variant=lvl$n
    case "$name" in
        III) stem=iii ;;
        V) stem=v ;;
        *) stem=$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]') ;;
    esac
    compile "$variant"
    i=0
    while [ "$i" -lt "$count" ]; do
        # SHA-256 of a consecutive counter, then the digest and its first
        # 16 bytes, so the exporter's 48-byte seed is determined by i.
        seed=$(printf 'shekyl-prism-accept-positive|%s|%s' "$name" "$i" | sha256sum | awk '{print $1}')
        scratch=$work/$variant-p$i
        python3 -c "import sys; raw=bytes.fromhex(sys.argv[1]); sys.stdout.buffer.write(raw+raw[:16])" "$seed" \
            | "$build/export_s5_$variant" keygen "$scratch" > "$scratch.pub"
        coefficient=$(awk '{print $2}' "$scratch.pub")
        pair=$(env -u CARGO_TARGET_DIR cargo run --quiet --manifest-path "$root/Cargo.toml" --bin s5_vector -- index "$name" "$coefficient")
        scalar=$(printf '%s\n' "$pair" | sed -n '2p')
        python3 -c "import sys; raw=bytes.fromhex(sys.argv[1]); sys.stdout.buffer.write(raw+raw[:16])" "$seed" \
            | "$build/export_s5_$variant" sign "$scratch" "$scalar" > "$scratch.sig"
        out=$root/prism-verify/testdata/accept/s5_${stem}_p$i.txt
        cat "$scratch.pub" "$scratch.sig" | env -u CARGO_TARGET_DIR cargo run --quiet --manifest-path "$root/Cargo.toml" --bin s5_vector -- commit "$name" "$out"
        printf 'wrote %s seed %s\n' "$(basename "$out")" "$i"
        i=$((i + 1))
    done
}

if [ "${1:-}" = "positive" ]; then
    positive "$2" "$3" "$4"
    exit
fi

one 1 I 17 17
one 1 I 1 1 s5_lvl1b.txt
one 3 III 48 18
one 3 III 8 20 s5_lvl3b.txt
one 5 V 17 22
one 5 V 3 18 s5_lvl5b.txt
