#!/bin/bash
# dudect fixed-vs-random Welch t-test per candidate on x86-64 (native),
# pinned to one core (RLC_CPU), serial. Uses the matrix binaries.
# Resumable: existing CSVs are kept. Output harness/results/raw/.
cd "$(dirname "$0")/harness"
W=${W:-/workspace/ctb-work}
export RLC_CPU=${RLC_CPU:-3}
CANDS=${CANDS:-"asm crate none weak bbmask bbuse noinline volatile subtle-vol subtle-bb cmov ctutils selunpred"}
mkdir -p results/raw
for id in ${IDS:-}; do :; done
for tc in ${TCS:-1.97.1}; do
 for cfg in ${CFGS:-c1}; do
  for c in $CANDS; do
   BIN=$W/bins/${tc}_x86_64_${cfg}_${c}
   [ -x $BIN ] || continue
   for sn in ${SETN:-p248:40000 p500:20000}; do
     s=${sn%:*}; n=${sn#*:}
     tag=_${c}_${tc}_${cfg}
     [ -s results/raw/dudect_${s}_ct${tag}.csv ] && continue
     RLC_TAG=$tag $BIN dudect $s $n ct 2>/dev/null
     echo "dudect $s $tag done"
   done
  done
 done
done
echo DUDECT DONE
