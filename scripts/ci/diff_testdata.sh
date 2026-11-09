#!/bin/bash
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
#
# Fail if a committed testdata directory differs from the worktree.
# An empty find is a failure: a diff of no paths is a pass that checked
# nothing.

set -euo pipefail
cd "$(dirname "$0")/../.."

mapfile -d '' dirs < <(find . -type d -name testdata \
    -not -path './external/*' -not -path './target/*' -not -path './.git/*' -print0)
if [ "${#dirs[@]}" -eq 0 ]; then
    echo "no testdata directory" >&2
    exit 1
fi
git diff --exit-code -- "${dirs[@]}"
