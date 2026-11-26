#!/usr/bin/env python3
"""Residual-stack-word scan of scripts/ci/stage2c_stack.py without the Callgrind
half (Valgrind is not installable on the Pi without root). The probe build, the
paint/guard window, the canary gate and the gdb scan are the repo's own code,
imported unchanged. Usage (from the repo root): stack_only.py SET [SET...]"""
import sys, tempfile
from collections import Counter
from pathlib import Path
ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / "scripts" / "ci"))
sys.path.insert(0, str(ROOT / "scripts"))
import stage2c_stack as S
from stage2c_fixtures import (CLASS_COUNTS, LIMB_BYTES, SET_WIDTHS, corpus, encode_basis,
                              nontrivial_words, retention_canary, self_test)
from stage2c_probe import ARCH, build_command, instrument_constants, release_libraries, run

def main(names):
    self_test()
    libraries = release_libraries()
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory)
        for name in names:
            width = SET_WIDTHS[name]
            binary = tmp / "probe"
            run(*build_command(libraries, name, binary))
            limits = instrument_constants(binary)
            markers = S.marker_addresses(run("nm", "-C", str(binary)))
            canary = retention_canary(width, limits["canary"])
            planted = nontrivial_words(canary, width)
            encoded = tmp / "basis.bin"
            encoded.write_bytes(canary)
            _, found, total = S.inspect(binary, markers, "canary", encoded, "sign-generic", planted, limits)
            assert (found, total) == (len(planted), len(planted)), f"canary {found}/{total}"
            print(f"{ARCH} {name} canary_gate {found}/{total}", flush=True)
            counts = Counter()
            for cls, values in corpus(name):
                basis = encode_basis(values[:9], width)
                words = nontrivial_words(basis, width)
                encoded.write_bytes(basis)
                depth, found, total = S.inspect(binary, markers, "stack", encoded, cls, words, limits)
                counts[cls] += 1
                print(f"{ARCH} {name} {cls} {counts[cls]} stack_bytes={depth} residual_words={found}/{total}", flush=True)

main(sys.argv[1:])
