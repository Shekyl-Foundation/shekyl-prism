#!/bin/bash
# Phase A: build every candidate x toolchain x arch x config (resumable:
# an existing binary is skipped; a failed build leaves a .fail marker).
# Uses -j2 (the box is shared). W=/workspace/ctb-work by default.
cd "$(dirname "$0")/harness"
export PATH=$HOME/.cargo/bin:$PATH
W=${W:-/workspace/ctb-work}
CANDS=${CANDS:-"asm none weak bbmask bbuse noinline volatile subtle-vol subtle-bb cmov ctutils selunpred crate"}
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
for tc in ${TCS:-1.97.1 1.85.1}; do
 for arch in ${ARCHS:-x86_64 aarch64}; do
  for cfg in ${CFGS:-c1 c2 c3 c4}; do
   case $cfg in
    c1) O=3; L=fat; CU=1; CPU="";;
    c2) O=2; L=fat; CU=1; CPU="";;
    c3) O=3; L=off; CU=16; CPU="";;
    c4) O=3; L=fat; CU=1; if [ $arch = x86_64 ]; then CPU="-C target-cpu=native"; else CPU="-C target-cpu=neoverse-v1"; fi;;
   esac
   if [ $arch = aarch64 ]; then
     # static, non-PIE: fixed addresses for qemu -dfilter (qemu-aarch64 maps PIE at random bases)
     RF="-C target-feature=+crt-static -C relocation-model=static $CPU"; TGT="--target aarch64-unknown-linux-gnu"; SUB=aarch64-unknown-linux-gnu/release
   else
     RF="$CPU"; TGT=""; SUB=release
   fi
   for c in $CANDS; do
    id=${tc}_${arch}_${cfg}_${c}
    [ -e $W/bins/$id ] || [ -e $W/bins/$id.fail ] && continue
    td=$W/tgt/$tc-$arch-$cfg
    if CARGO_PROFILE_RELEASE_OPT_LEVEL=$O CARGO_PROFILE_RELEASE_LTO=$L CARGO_PROFILE_RELEASE_CODEGEN_UNITS=$CU \
       RUSTFLAGS="$RF" cargo +$tc build --release -j2 --bin rlc --features b-$c $TGT --target-dir $td > $W/logs/build_$id.log 2>&1; then
      cp $td/$SUB/rlc $W/bins/$id && echo "built $id"
    else
      touch $W/bins/$id.fail; echo "FAILED $id"
    fi
   done
  done
 done
done
echo BUILDS DONE
