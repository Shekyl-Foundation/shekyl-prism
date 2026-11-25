#!/bin/bash
# Valgrind memcheck taint check (ctgrind method) per candidate, x86-64
# only: T, x, y marked undefined before zi::reduce; memcheck reports any
# branch or address that depends on them. Static build (crt-static).
# Resumable: an existing results/ctgrind/<id>.txt is kept.
cd "$(dirname "$0")/harness"
export PATH=$HOME/.cargo/bin:$PATH
W=${W:-/workspace/ctb-work}
OUT=../results/ctgrind; mkdir -p $OUT $W/logs
CANDS=${CANDS:-"asm none weak bbmask bbuse noinline volatile subtle-vol subtle-bb cmov ctutils selunpred crate"}
for tc in ${TCS:-1.97.1 1.85.1}; do
 for cfg in ${CFGS:-c1}; do
  case $cfg in
   c1) O=3; L=fat; CU=1; CPU="";;
   c2) O=2; L=fat; CU=1; CPU="";;
   c3) O=3; L=off; CU=16; CPU="";;
   c4) O=3; L=fat; CU=1; CPU="-C target-cpu=native";;
  esac
  for c in $CANDS; do
   id=${tc}_${cfg}_${c}
   [ -s $OUT/$id.txt ] && continue
   td=$W/tgt/ctg-$tc-$cfg
   if ! VALGRIND_INC=/usr/include CARGO_PROFILE_RELEASE_OPT_LEVEL=$O CARGO_PROFILE_RELEASE_LTO=$L CARGO_PROFILE_RELEASE_CODEGEN_UNITS=$CU \
      RUSTFLAGS="-C target-feature=+crt-static $CPU" cargo +$tc build --release -j2 --bin rlc --features "ctgrind b-$c" \
      --target x86_64-unknown-linux-gnu --target-dir $td > $W/logs/ctgbuild_$id.log 2>&1; then
     echo "build failed (see log; e.g. MSRV)" > $OUT/$id.txt; continue
   fi
   BIN=$td/x86_64-unknown-linux-gnu/release/rlc
   for s in p248 p324 p376 p500 p664; do
    log=$W/logs/ctgrind_${id}_$s.log
    valgrind --tool=memcheck --track-origins=yes --num-callers=30 $BIN ctgrind $s ${RUNS:-2} > $log 2>&1
    python3 ../parse_ctgrind.py "$log" "$s"
   done > $OUT/$id.txt.tmp && mv $OUT/$id.txt.tmp $OUT/$id.txt
   echo "ctgrind $id: $(grep -c 'in_ct_code=0' $OUT/$id.txt)/5 sets clean"
  done
 done
done
# positive control: a deliberate secret branch, on the asm build
id=1.97.1_c1_asm_control
if [ ! -s $OUT/$id.txt ]; then
  BIN=$W/tgt/ctg-1.97.1-c1/x86_64-unknown-linux-gnu/release/rlc
  VALGRIND_INC=/usr/include RUSTFLAGS="-C target-feature=+crt-static" cargo +1.97.1 build -q --release -j2 --bin rlc --features "ctgrind b-asm" --target x86_64-unknown-linux-gnu --target-dir $W/tgt/ctg-1.97.1-c1 >/dev/null 2>&1
  RLC_CTGRIND_CONTROL=1 valgrind --tool=memcheck --track-origins=yes --num-callers=30 $BIN ctgrind p248 2 > $W/logs/ctgrind_$id.log 2>&1
  python3 ../parse_ctgrind.py $W/logs/ctgrind_$id.log p248 > $OUT/$id.txt
fi
echo CTGRIND DONE
