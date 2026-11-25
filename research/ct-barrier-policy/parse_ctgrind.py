# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Parse one memcheck log (from research/rust-lagrange-ct/ctgrind/run.sh,
unchanged logic): count error contexts, drop static-glibc allocator
false positives, count those in the constant-time code. The positive
control's deliberate branch is in rlc's ctgrind_set, so it counts under
outside_glibc, not in_ct_code."""
import re
import sys

log, s = sys.argv[1], sys.argv[2]
txt = open(log).read()
blocks = re.split(r'\n==\d+== \n', txt)
errs = [b for b in blocks if re.search(r'(Conditional jump|Use of uninitialised|Invalid)', b)]
GLIBC = re.compile(r'at 0x[0-9A-F]+: (free|brk|sbrk|malloc|realloc|calloc|_int_\w+|sysmalloc|__mem\w+|__str\w+|__libc\w+|_dl_\w+)\b')


def top(b):
    m = re.search(r'(?:at|by) 0x[0-9A-F]+: .*', b)
    return m.group(0) if m else '?'


real = [b for b in errs if not GLIBC.search(top(b))]
in_ct = [b for b in real if 'zi::reduce' in b or 'ct.rs' in b or 'zi.rs' in b or 'prism_ct_barrier' in b
         or 'subtle' in b or 'cmov' in b or 'ctutils' in b]
done = re.search(r'ctgrind: (\d+) runs done', txt)
print(f"{s}: runs={done.group(1) if done else 'NOT FINISHED'} memcheck_error_contexts={len(errs)} "
      f"static_glibc_allocator_false_positives={len(errs) - len(real)} outside_glibc={len(real)} in_ct_code={len(in_ct)}")
for b in real[:3]:
    kind = re.search(r'(Conditional jump[^\n]*|Use of uninitialised[^\n]*|Invalid[^\n]*)', b).group(1)
    print(f'  {kind} | {top(b)}')
