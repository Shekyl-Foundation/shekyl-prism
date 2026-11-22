#!/bin/sh
# Cycle benchmark and dudect runs, pinned to one core (RLC_CPU, default 6).
# Serial on purpose: one measurement at a time.
cd "$(dirname "$0")"
export RLC_CPU=${RLC_CPU:-6}
BIN=target/gmp/release/rlc
mkdir -p results/raw
if [ ! -s results/bench.csv ]; then
  echo "set,impl,reps,median_cycles,mean_cycles,p999_cycles,cv,mean_steps" > results/bench.csv.tmp
  for sr in p248:2000 p324:1500 p376:1000 p500:600 p664:400; do
    $BIN bench ${sr%:*} ${sr#*:} >> results/bench.csv.tmp
  done
  mv results/bench.csv.tmp results/bench.csv
fi
for sn in p248:100000 p324:100000 p376:60000 p500:50000 p664:40000; do
  s=${sn%:*}; n=${sn#*:}
  [ -s results/raw/dudect_${s}_ct.csv ] || $BIN dudect $s $n ct
  [ -s results/raw/dudect_${s}_gauss.csv ] || $BIN dudect $s 20000 gauss
done
echo MEASURE DONE
