#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Integer-divide mnemonics in a release listing.

x86 integer division is `div` or `idiv` with an optional width suffix.
AArch64 integer division is `udiv` or `sdiv`. A floating-point divide
(`divsd`, `fdiv`) is a different instruction and is not this predicate.
The shift trace and the reducer profile both call `hardware_divide`.
"""
import re

# Width suffixes are the single letters objdump appends (`divq`, `idivl`).
# A second letter, as in `divsd`, must not match.
_DIVIDE = re.compile(r"(?:i?div[bwlq]?|[us]div)\Z")
# Spaces and tabs only. `\s` would cross the newline and swallow the next
# instruction into this one's operands.
_INSTRUCTION = re.compile(r"(?m)^[ \t]*([0-9a-f]+):[ \t]+(\S+)(?:[ \t]+(.*))?$")


def hardware_divide(mnemonic):
    """True when `mnemonic` is an integer hardware divide."""
    return _DIVIDE.fullmatch(mnemonic) is not None


def instructions(listing):
    """Each objdump instruction as `(address, mnemonic, operands)`."""
    for address, mnemonic, operands in _INSTRUCTION.findall(listing):
        yield int(address, 16), mnemonic, operands


def mnemonics(listing):
    """Map a program counter to the mnemonic objdump printed for it."""
    return {address: mnemonic for address, mnemonic, _ in instructions(listing)}


def self_test():
    """The predicate accepts integer divides and refuses floating-point ones."""
    for opcode in ("div", "divq", "idiv", "idivq", "udiv", "sdiv"):
        if not hardware_divide(opcode):
            raise AssertionError(f"missed integer divide {opcode}")
    for opcode in ("divsd", "divss", "divps", "fdiv", "mul", "cmov", "b"):
        if hardware_divide(opcode):
            raise AssertionError(f"treated {opcode} as an integer divide")
    sample = "  401000:\tudiv x1, x2, x3\n  401004:\tadd x1, x1, x2\n  401008:\tret\n  401009:\tint3\n"
    parsed = list(instructions(sample))
    if parsed != [
        (0x401000, "udiv", "x1, x2, x3"),
        (0x401004, "add", "x1, x1, x2"),
        (0x401008, "ret", ""),
        (0x401009, "int3", ""),
    ]:
        raise AssertionError(parsed)
    if mnemonics(sample) != {pc: mnemonic for pc, mnemonic, _ in parsed}:
        raise AssertionError(mnemonics(sample))


if __name__ == "__main__":
    self_test()
    print("stage2c isa self-test passed")
