#!/usr/bin/env bash
# tools/scaffold.sh — Create the canonical stateless-architecture directory tree
#
# Creates the repo layout that the maintenance skills and the pattern
# language assume. Safe to re-run: mkdir -p is idempotent and existing
# files are never overwritten.
#
#   book/                mdBook pattern language (chapters 01-09 + SUMMARY.md)
#   spec/                formal artifacts: state-blob.schema.json,
#                        service-bus.proto, lifecycle.fsm, progressive-restore.md
#   reference/rust/src/  reference implementation crate sources
#   reference/tests/     integration tests (minimal_demo_test.rs, integration_test.rs)
#   rfcs/                numbered RFCs (0001-...)
#   tools/               helper scripts (validate-schema.sh, scaffold.sh)
#   .github/workflows/   CI + Pages deploy workflows
#
# Usage: ./tools/scaffold.sh [--in DIR] [--git]
#   --in DIR   create the tree under DIR (default: current directory)
#   --git      git init the target unless it is already a repository

set -euo pipefail

TARGET="."
INIT_GIT=0

usage() {
    sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'
    echo
    echo "Options:"
    echo "  --in DIR   target directory (default: .)"
    echo "  --git      git init the target if not already a repository"
    echo "  -h, --help show this message"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --in)
            TARGET="$2"
            shift 2
            ;;
        --git)
            INIT_GIT=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Error: unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [ ! -d "$TARGET" ]; then
    mkdir -p "$TARGET"
fi

cd "$TARGET"

DIRS=(
    book
    spec
    reference/rust/src
    reference/tests
    rfcs
    tools
    .github/workflows
)

for d in "${DIRS[@]}"; do
    mkdir -p "$d"
done

# Keep empty dirs visible to git until real files land.
find book spec reference rfcs tools .github -type d -empty -exec touch {}/.gitkeep \;

if [ "$INIT_GIT" -eq 1 ] && [ ! -d .git ]; then
    if ! git init -b main > /dev/null 2>&1; then
        # git older than 2.28: init then rename the default branch.
        git init > /dev/null
        git checkout -b main > /dev/null 2>&1 || true
    fi
    echo "Initialized git repository ($(pwd))"
fi

echo "Created stateless-architecture tree in: $(pwd)"
echo
find . -type d \( -name .git -prune -o -print \) | sed 's|^\./||' | sort | sed -e 's|^|  |'