#!/bin/bash
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Fail if a generator changed the committed test data, or wrote some
# where the data does not live.
#
# The data is under prism-vectors/data. After a generator has run:
#
# - no tracked file there differs from the commit;
# - no untracked file is there (a generator that writes a new name);
# - no directory named testdata exists (a generator that still writes to
#   a place the data left on 2026-11-16).
#
# A missing data directory is a failure: a diff of no paths is a pass
# that checked nothing.

set -euo pipefail
cd "$(dirname "$0")/../.."

data=prism-vectors/data
if [ ! -d "$data" ]; then
    echo "no $data directory" >&2
    exit 1
fi
tracked=$(git ls-files -- "$data" | wc -l)
if [ "$tracked" -eq 0 ]; then
    echo "no tracked file under $data" >&2
    exit 1
fi
git diff --exit-code -- "$data"
untracked=$(git ls-files --others --exclude-standard -- "$data")
if [ -n "$untracked" ]; then
    echo "files under $data that are not committed:" >&2
    echo "$untracked" >&2
    exit 1
fi
stale=$(find . -type d -name testdata \
    -not -path './external/*' -not -path './target/*' -not -path './fuzz/target/*' \
    -not -path './.git/*' -print)
if [ -n "$stale" ]; then
    echo "a testdata directory exists; the data is under $data:" >&2
    echo "$stale" >&2
    exit 1
fi
echo "$tracked committed files under $data are unchanged; nothing was written beside them"
