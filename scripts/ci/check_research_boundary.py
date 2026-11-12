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
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FEATURE = "research-reduced-width"


def main():
    verifier = tomllib.loads((ROOT / "prism-verify/Cargo.toml").read_text())
    if verifier["features"].get("default") != []:
        raise SystemExit("verifier default features must be empty")
    for name in ("prism-field", "prism-isogeny", "prism-verify", "prism-signer"):
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
    print("Default verifier API excludes research; explicit opt-in works; production manifests are isolated.")


if __name__ == "__main__":
    main()
