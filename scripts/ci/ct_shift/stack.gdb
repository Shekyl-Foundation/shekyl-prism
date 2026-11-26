# Copyright (c) 2025-2026, The Shekyl Foundation
# All rights reserved.
# BSD-3-Clause
# start/end are linked addresses supplied by the runner. Only public fixtures.
set pagination off
set confirm off
set disable-randomization off
break *$start
break *$end
run
python
import gdb, os
assert int(gdb.parse_and_eval("$pc")) == int(gdb.parse_and_eval("$start"))
sp = int(gdb.parse_and_eval("$sp"))
inferior = gdb.selected_inferior()
size = 224 * 1024
bottom = sp - size
paint = bytes([0xa5]) * size
inferior.write_memory(bottom, paint)
end
continue
python
assert int(gdb.parse_and_eval("$pc")) == int(gdb.parse_and_eval("$end"))
after = bytes(inferior.read_memory(bottom, size))
changed = next(i for i in range(size) if after[i] != paint[i])
assert changed >= 4096, "probe reached the lower guard page of the measurement"
encoded = open(os.environ["PRISM_STACK_FIXTURE"], "rb").read()
stride = len(encoded) // 9
# Only nontrivial aligned input words; skip repeated padding and small integers.
words = {encoded[i+j:i+j+8] for i in range(0, len(encoded), stride)
         for j in range(0, stride-1, 8) if len(set(encoded[i+j:i+j+8])) >= 5}
assert words, "no distinct nontrivial words in fixture"
gdb.write("stack-result %d %d %d\n" % (size-changed, sum(w in after for w in words), len(words)))
end
