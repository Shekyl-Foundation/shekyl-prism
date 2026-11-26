# Copyright (c) 2025-2026, The Shekyl Foundation
# All rights reserved.
# BSD-3-Clause
# The runner supplies $start, $end, $paint and $guard, and the word file
# PRISM_STACK_WORDS (one little-endian limb per line, hex). The two Python
# blocks share GDB's one interpreter, so the second reads the first's names.
# Only public fixtures.
set pagination off
set confirm off
set disable-randomization off
break *$start
break *$end
run
python
import gdb
assert int(gdb.parse_and_eval("$pc")) == int(gdb.parse_and_eval("$start"))
sp = int(gdb.parse_and_eval("$sp"))
paint = int(gdb.parse_and_eval("$paint"))
guard = int(gdb.parse_and_eval("$guard"))
# Ink that no reducer stores as a deliberate fill. A changed byte is a write.
PAINT_BYTE = 0xA5
inferior = gdb.selected_inferior()
bottom = sp - paint
ink = bytes([PAINT_BYTE]) * paint
inferior.write_memory(bottom, ink)
end
continue
python
import os
assert int(gdb.parse_and_eval("$pc")) == int(gdb.parse_and_eval("$end"))
after = bytes(inferior.read_memory(bottom, paint))
changed = next((i for i in range(paint) if after[i] != ink[i]), None)
if changed is None:
    raise RuntimeError("painted stack was not written")
if changed < guard:
    raise RuntimeError("stack write reached the guard page; raise STACK_PAINT_BYTES")
words = [bytes.fromhex(line.decode().strip())
         for line in open(os.environ["PRISM_STACK_WORDS"], "rb") if line.strip()]
if not words:
    raise RuntimeError("no words to search for")
found = sum(word in after for word in words)
gdb.write("stack-result %d %d %d\n" % (paint - changed, found, len(words)))
end
