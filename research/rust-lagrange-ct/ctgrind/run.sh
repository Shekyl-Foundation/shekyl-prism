#!/bin/sh
# Constant-time check with Valgrind memcheck (the ctgrind method): T, x, y
# are marked undefined; memcheck then reports any branch or memory address
# that depends on them. Statically linked so memcheck needs no ld.so
# debug symbols (static glibc's own start-up and allocator then give a few
# known false positives, which are not in this crate and are filtered out
# by stack). VG=<valgrind prefix> (here a micromamba env).
# RLC_CTGRIND_CONTROL=1 adds a deliberate secret branch (positive control);
# RLC_FEATURES=weak-barrier builds the first, too-weak barrier (a real
# compiler-made branch, second positive control).
cd "$(dirname "$0")/.."
VG=${VG:-/workspace/mamba/envs/vg}
VALGRIND_INC=$VG/include RUSTFLAGS="-C target-feature=+crt-static" \
  cargo build -q --release -j2 --features "ctgrind $RLC_FEATURES" --target-dir target/ctgrind$RLC_FEATURES --target x86_64-unknown-linux-gnu || exit 1
BIN=target/ctgrind$RLC_FEATURES/x86_64-unknown-linux-gnu/release/rlc
mkdir -p results/raw
for s in ${SETS:-p248 p324 p376 p500 p664}; do
  log=results/raw/ctgrind_${s}${RLC_CTGRIND_CONTROL:+_control}${RLC_FEATURES:+_$RLC_FEATURES}.log
  $VG/bin/valgrind --tool=memcheck --track-origins=yes --num-callers=30 $BIN ctgrind $s ${RUNS:-3} > $log 2>&1
  python3 - "$log" "$s" <<'PY'
import re, sys
log, s = sys.argv[1], sys.argv[2]
txt = open(log).read()
blocks = re.split(r'\n==\d+== \n', txt)
errs = [b for b in blocks if re.search(r'(Conditional jump|Use of uninitialised|Invalid)', b)]
GLIBC = re.compile(r'at 0x[0-9A-F]+: (free|brk|sbrk|malloc|realloc|calloc|_int_\w+|sysmalloc|__mem\w+|__str\w+|__libc\w+|_dl_\w+)\b')
def top(b):
    m = re.search(r'(?:at|by) 0x[0-9A-F]+: .*', b)
    return m.group(0) if m else '?'
real = [b for b in errs if not GLIBC.search(top(b))]
in_reduce = [b for b in real if 'zi::reduce' in b or 'ct.rs' in b or 'zi.rs' in b]
done = re.search(r'ctgrind: (\d+) runs done', txt)
print(f"{s}: runs={done.group(1) if done else 'NOT FINISHED'} memcheck_error_contexts={len(errs)} "
      f"static_glibc_allocator_false_positives={len(errs) - len(real)} outside_glibc={len(real)} in_ct_code={len(in_reduce)}")
for b in real[:3]:
    kind = re.search(r'(Conditional jump[^\n]*|Use of uninitialised[^\n]*|Invalid[^\n]*)', b).group(1)
    print(f'  {kind} | {top(b)}')
PY
done
