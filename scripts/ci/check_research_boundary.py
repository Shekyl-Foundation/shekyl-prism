#!/usr/bin/env python3
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Prove the default verifier has no reduced-width entry point.

The isolated consumer must compile with the real verifier, fail specifically
on importing its research module, then compile only with the explicit
research feature. Production dependencies may not enable that feature or
import prism-compare. This tests API absence, not just a source grep.

The front crate `shekyl-prism` is production code under the same rules. It
declares no feature at all, and the verifier in its resolved build graph
has the research feature off; the graph reader is shown to see the feature
when it is requested.

The testkit `prism-testkit` is not production code, but the front crate's
tests are built with it, so it is held to the same two rules: no feature
table, and the verifier in its default graph has the research feature off.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FEATURE = "research-reduced-width"


def verifier_features(package, *extra):
    """The features `prism-verify` has in the build graph of `package`."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "-p", package, "-e", "normal,build",
         "--prefix", "none", "-f", "{p}\t{f}", *extra],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise SystemExit(f"cargo tree failed for {package}:\n" + result.stderr)
    found = None
    for line in result.stdout.splitlines():
        node, _, listed = line.partition("\t")
        if node.split()[:1] == ["prism-verify"]:
            names = listed.replace(" (*)", "").split(",")
            found = (found or set()) | {name for name in names if name}
    if found is None:
        raise SystemExit(f"prism-verify is not in the build graph of {package}")
    return found


def main():
    verifier = tomllib.loads((ROOT / "prism-verify/Cargo.toml").read_text())
    if verifier["features"].get("default") != []:
        raise SystemExit("verifier default features must be empty")
    front = tomllib.loads((ROOT / "shekyl-prism/Cargo.toml").read_text())
    if "features" in front:
        raise SystemExit("the front crate declares features; it must declare none")
    if FEATURE in verifier_features("shekyl-prism"):
        raise SystemExit("the front crate's build has the research feature on the verifier")
    testkit = tomllib.loads((ROOT / "prism-testkit/Cargo.toml").read_text())
    if "features" in testkit:
        raise SystemExit("the testkit declares features; it must declare none")
    if FEATURE in verifier_features("prism-testkit"):
        raise SystemExit("the testkit's build has the research feature on the verifier")
    if FEATURE not in verifier_features("prism-verify", "--features", f"prism-verify/{FEATURE}"):
        raise SystemExit("the graph reader does not see the research feature when it is requested")
    for name in (
        "prism-field", "prism-isogeny", "prism-verify", "prism-signer", "shekyl-prism",
        "prism-testkit",
    ):
        text = (ROOT / name / "Cargo.toml").read_text()
        manifest = tomllib.loads(text)
        for key, dep in manifest.get("dependencies", {}).items():
            package = dep.get("package", key) if isinstance(dep, dict) else key
            if package == "prism-compare":
                raise SystemExit(f"production {name} depends on comparison code")
            if isinstance(dep, dict) and FEATURE in dep.get("features", []):
                raise SystemExit(f"production {name} enables the research feature")
        if name == "prism-signer" and FEATURE in text:
            raise SystemExit("signer manifest mentions the research feature")
    env = dict(os.environ)
    env.pop("CARGO_TARGET_DIR", None)
    env["CARGO_BUILD_JOBS"] = "2"
    with tempfile.TemporaryDirectory(prefix="prism-api-boundary-") as directory:
        tmp = Path(directory)
        (tmp / "src").mkdir()
        path = json.dumps(str(ROOT / "prism-verify"))
        (tmp / "Cargo.toml").write_text(
            '[package]\nname="prism-api-boundary"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[features]\nexperiments=["prism-verify/research-reduced-width"]\n'
            f'[dependencies]\nprism-verify={{path={path},default-features=false}}\n'
        )
        command = ["cargo", "check", "--offline", "--manifest-path", str(tmp / "Cargo.toml")]
        source = tmp / "src/lib.rs"
        source.write_text("pub use prism_verify::verify;\n")
        normal = subprocess.run(command, env=env, text=True, capture_output=True)
        if normal.returncode:
            raise SystemExit("default verifier consumer failed to build:\n" + normal.stderr)
        source.write_text("pub use prism_verify::research::verify_scalar;\n")
        absent = subprocess.run(command, env=env, text=True, capture_output=True)
        if absent.returncode == 0 or "could not find `research` in `prism_verify`" not in absent.stderr:
            raise SystemExit("research API absence not established:\n" + absent.stderr)
        enabled = subprocess.run(command + ["--features", "experiments"], env=env, text=True, capture_output=True)
        if enabled.returncode:
            raise SystemExit("explicit research consumer failed:\n" + enabled.stderr)
    print(
        "Default verifier API excludes research; explicit opt-in works; production "
        "manifests are isolated; the front crate and the testkit declare no "
        "feature and build the verifier without it."
    )


if __name__ == "__main__":
    main()
