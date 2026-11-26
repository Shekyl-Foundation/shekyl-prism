#!/bin/bash
# Runs on the Pi as skl-agent. Everything lives under ~/shekyl-timing.
# Own RUSTUP_HOME/CARGO_HOME so the host's ~/.rustup and ~/.cargo are untouched.
# Usage: setup_build.sh BRANCH_WITH_PATCH
set -eu
BRANCH=${1:-experiment/p248-arm-timing}
W=$HOME/shekyl-timing
mkdir -p "$W/results" "$W/logs"
cd "$W"
export RUSTUP_HOME=$W/rustup CARGO_HOME=$W/cargo
export PATH=$CARGO_HOME/bin:$PATH
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
  curl -sSf https://sh.rustup.rs -o rustup-init.sh
  sh rustup-init.sh -y --profile minimal --default-toolchain none --no-modify-path
fi
[ -d src/.git ] || git clone --quiet https://github.com/Shekyl-Foundation/shekyl-prism src
cd src
git fetch --quiet origin dev "$BRANCH"
# Pristine tree at the dev tip the report names, then the harness patch on top.
DEV=$(git rev-parse origin/dev)
git checkout --quiet --detach --force "$DEV"
git clean -fdq -e target
git show "origin/$BRANCH:research/p248-arm-timing/harness.patch" > "$W/harness.patch"
git apply "$W/harness.patch"
echo "dev=$DEV" | tee "$W/logs/build_provenance.txt"
rustup toolchain install "$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)" --profile minimal
rustc --version | tee -a "$W/logs/build_provenance.txt"
( time cargo build --release --locked -p shekyl-prism --features signer --example stage2c ) 2>&1 | tail -20
cp target/release/examples/stage2c "$W/stage2c-dev-patched"
sha256sum "$W/stage2c-dev-patched" | tee -a "$W/logs/build_provenance.txt"
echo BUILD_DONE
