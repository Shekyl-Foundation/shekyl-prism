#!/bin/bash
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

# Install distribution packages on a CI runner, trying again when the
# package mirrors cannot be reached.
#
#   bash scripts/ci/apt_install.sh clang
#
# A hosted runner now and then cannot resolve its mirrors for a minute or
# two ("Temporary failure resolving 'archive.ubuntu.com'"). One such
# failure in one job made a whole nightly red on 2026-11-11. `apt-get
# update` reports that as a warning and exits 0, so the index refresh and
# the install are retried together.
set -euo pipefail
if [ "$#" -eq 0 ]; then
    echo 'usage: apt_install.sh <package>...' >&2
    exit 2
fi
attempts=5
for attempt in $(seq 1 "$attempts"); do
    if sudo apt-get -o Acquire::Retries=3 update \
        && sudo apt-get -o Acquire::Retries=3 install -y --no-install-recommends "$@"; then
        exit 0
    fi
    echo "apt attempt ${attempt} of ${attempts} failed" >&2
    if [ "$attempt" -lt "$attempts" ]; then
        sleep $((attempt * 30))
    fi
done
echo "could not install: $*" >&2
exit 1
