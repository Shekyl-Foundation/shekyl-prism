#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""The public surface of `prism-int` is the committed list, and no more.

`prism-int/surface.txt` lists every public item of the crate: each `pub`
function, type, constant and re-export, each trait implementation and each
derive, in the order of the sources. A change to the surface that does not
change that file in the same commit fails here, so a reviewer sees every
new way in and every new way out as a line of a diff.

1. `prism-int/src/lib.rs` and `prism-int/src/secret.rs` exist, and the
   list read from the sources has the two secret types and their three
   exits in it (the subject exists).
2. The list read from the sources is the committed file, byte for byte.
3. No trait that would let a secret out is implemented or derived for
   `SecretInt` or `SecretBit`: `PartialEq`, `Eq`, `PartialOrd`, `Ord`,
   `Hash`, `Display`, `Copy`, `AsRef`, `Borrow`, `Deref`, `Into`, a
   `From` whose source is a secret type, the formatting traits other than
   `Debug`, and `Serialize`. The compile-fail tests of the crate hold the
   same promises from the outside; this holds them where they are written.

Test modules are left out: the list stops at `#[cfg(test)]` in each file.

`--write` rewrites the committed file. `--selftest` plants each breach.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCES = "prism-int/src"
LIST = "prism-int/surface.txt"
STARTS = ("pub fn ", "pub const fn ", "pub const ", "pub struct ", "pub enum ", "pub use ", "pub mod ",
          "pub trait ", "pub type ", "pub static ", "impl", "#[derive(")
SECRETS = ("SecretInt", "SecretBit")
FORBIDDEN = ("PartialEq", "Eq", "PartialOrd", "Ord", "Hash", "Display", "Copy", "AsRef", "Borrow",
             "Deref", "DerefMut", "Into", "Serialize", "LowerHex", "UpperHex", "Binary", "Octal",
             "LowerExp", "UpperExp", "Pointer")
SUBJECT = ("pub struct SecretInt<const N: usize>", "pub struct SecretBit", "pub fn declassify(&self)",
           "pub fn export_le_bytes(")


def items(text):
    """The public items of one source file, each as one line."""
    found = []
    lines = text.split("#[cfg(test)]\nmod ")[0].splitlines()
    index = 0
    variants = False
    while index < len(lines):
        stripped = lines[index].strip()
        if variants:
            # The variants of a public enum are public.
            if stripped == "}":
                variants = False
            elif stripped and not stripped.startswith(("//", "#[")):
                found.append("        " + stripped.rstrip(","))
            index += 1
            continue
        if not stripped.startswith(STARTS):
            index += 1
            continue
        item = stripped
        whole = stripped.startswith("pub use ")
        while not item.endswith(";") and not item.endswith(")]") and (whole or "{" not in item):
            index += 1
            item += " " + lines[index].strip()
        if not whole:
            item = item.split("{")[0]
        item = re.sub(r"\s+", " ", item).strip().rstrip(";").removesuffix(" =")
        item = item.replace("( ", "(").replace(", )", ")")
        indent = "" if item.startswith(("impl", "#[derive(", "pub struct", "pub enum", "pub use", "pub mod")) else "    "
        found.append(indent + item)
        variants = item.startswith("pub enum ")
        index += 1
    return found


def surface(texts):
    """The list, from `{file name: text}`."""
    out = ["# The public surface of prism-int. Written by scripts/ci/check_int_surface.py --write;",
           "# CI holds the sources to it."]
    for name in sorted(texts):
        found = items(texts[name])
        if found:
            out.append("")
            out.append(f"[{name}]")
            out += found
    return "\n".join(out) + "\n"


def forbidden_fault(listing):
    """Rule 3, read from the list."""
    lines = listing.splitlines()
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("#[derive("):
            following = lines[index + 1].strip() if index + 1 < len(lines) else ""
            if any(f"struct {secret}" in following for secret in SECRETS):
                derived = [name.strip() for name in stripped[len("#[derive("):-2].split(",")]
                bad = [name for name in derived if name.split("::")[-1] in FORBIDDEN]
                if bad:
                    return f"`{following}` derives {', '.join(bad)}"
        match = re.match(r"impl(?:<[^>]*>)? (.+) for (.+)$", stripped)
        if not match:
            continue
        trait, target = match.group(1), match.group(2)
        name = re.split(r"[<\s]", trait.split("::")[-1])[0]
        if any(target.startswith(secret) for secret in SECRETS) and name in FORBIDDEN:
            return f"`{stripped}` lets a secret out"
        if name == "From" and any(secret in trait for secret in SECRETS):
            return f"`{stripped}` converts a secret type by `From`"
    return None


def fault(texts, committed):
    """The first breach, or None."""
    for required in ("lib.rs", "secret.rs"):
        if required not in texts:
            return f"{SOURCES}/{required} is absent"
    listing = surface(texts)
    for wanted in SUBJECT:
        if wanted not in listing:
            return f"the surface read from the sources has no `{wanted}`"
    problem = forbidden_fault(listing)
    if problem:
        return problem
    if listing != committed:
        return (f"{LIST} is not the surface of the sources. If the change is meant, run "
                "`python3 scripts/ci/check_int_surface.py --write` and commit the file")
    return None


def read():
    return {path.name: path.read_text() for path in sorted((ROOT / SOURCES).glob("*.rs"))}


def selftest():
    texts = read()
    committed = surface(texts)
    secret = texts["secret.rs"]

    def with_secret(text):
        changed = dict(texts)
        changed["secret.rs"] = text
        return changed, surface({**texts, "secret.rs": text})

    cases = [("the sources as they are", texts, committed, None)]
    added = secret.replace("impl<const N: usize> SecretInt<N> {", "impl<const N: usize> SecretInt<N> {\n    /// x\n    pub fn peek(&self) -> u64 {\n        self.limbs[0]\n    }\n", 1)
    cases.append(("a new public function", with_secret(added)[0], committed, "is not the surface"))
    for label, plant, expected in (
        ("PartialEq for the secret integer", "impl<const N: usize> PartialEq for SecretInt<N> {\n}\n", "lets a secret out"),
        ("Display for the secret bit", "impl fmt::Display for SecretBit {\n}\n", "lets a secret out"),
        ("From a secret into the public type", "impl<const N: usize> From<SecretInt<N>> for Int<N> {\n}\n", "by `From`"),
        ("From a secret reference", "impl<const N: usize> From<&SecretInt<N>> for Int<N> {\n}\n", "by `From`"),
        ("Deref for the secret integer", "impl<const N: usize> core::ops::Deref for SecretInt<N> {\n}\n", "lets a secret out"),
        ("AsRef for the secret integer", "impl<const N: usize> AsRef<Int<N>> for SecretInt<N> {\n}\n", "lets a secret out"),
        ("Copy for the secret bit", "impl Copy for SecretBit {\n}\n", "lets a secret out"),
    ):
        marker = "impl<const N: usize> Drop for SecretInt<N> {"
        changed, listing = with_secret(secret.replace(marker, plant + "\n" + marker, 1))
        cases.append((label, changed, listing, expected))
    derived = secret.replace("pub struct SecretBit {", "#[derive(PartialEq, Eq)]\npub struct SecretBit {", 1)
    changed, listing = with_secret(derived)
    cases.append(("a derive on the secret bit", changed, listing, "derives"))
    renamed = secret.replace("pub fn export_le_bytes(", "pub fn export_bytes(")
    changed, listing = with_secret(renamed)
    cases.append(("an exit renamed", changed, listing, "has no"))
    cases.append(("a file gone", {"lib.rs": texts["lib.rs"]}, committed, "absent"))
    failed = 0
    for label, sources, listing, expected in cases:
        got = fault(sources, listing)
        if (got is None) != (expected is None) or (expected and expected not in got):
            print(f"selftest: {label}: got {got!r}", file=sys.stderr)
            failed = 1
    if not failed:
        print(f"selftest: {len(cases)} cases")
    return failed


def main():
    arguments = sys.argv[1:]
    if arguments == ["--selftest"]:
        return selftest()
    texts = read()
    if arguments == ["--write"]:
        (ROOT / LIST).write_text(surface(texts))
        print(f"wrote {LIST}")
        return 0
    if arguments:
        print("usage: check_int_surface.py [--write | --selftest]", file=sys.stderr)
        return 2
    path = ROOT / LIST
    if not path.is_file():
        print(f"{LIST} is absent", file=sys.stderr)
        return 1
    problem = fault(texts, path.read_text())
    if problem:
        print(problem, file=sys.stderr)
        return 1
    print(f"{LIST}: {len(path.read_text().splitlines())} lines, as the sources")
    return 0


if __name__ == "__main__":
    sys.exit(main())
