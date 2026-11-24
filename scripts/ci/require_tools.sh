#!/bin/sh
# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

# Require programs a job needs and cannot install.
#
#   sh scripts/ci/require_tools.sh cc make cmake
#
# The Foundation's own runners give a job a container with no root and no
# package manager, so a job there cannot install what it lacks: what it
# needs is in the runner's image. This says so before the job's work
# starts. It names every program that is missing, not only the first, and
# prints where each one that is present was found and its first line of
# `--version`, so the log of a run records what it was built with.
#
# A job on a hosted runner installs with `scripts/ci/apt_install.sh`.
set -eu
if [ "$#" -eq 0 ]; then
    echo 'usage: require_tools.sh <program>...' >&2
    exit 2
fi
missing=''
for tool in "$@"; do
    if path=$(command -v "$tool"); then
        version=$("$tool" --version 2>&1 | sed -n 1p) || version='(no --version)'
        echo "$tool: $path: $version"
    else
        missing="$missing $tool"
    fi
done
if [ -n "$missing" ]; then
    echo "not in this runner's image:$missing" >&2
    exit 1
fi
