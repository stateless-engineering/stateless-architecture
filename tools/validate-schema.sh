#!/usr/bin/env bash
# tools/validate-schema.sh — Validate a state blob against the architecture spec
#
# Usage: ./tools/validate-schema.sh <blob.json>
#
# Requires: ajv-cli (npm install -g ajv-cli)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPEC_DIR="$SCRIPT_DIR/../spec"
SCHEMA="$SPEC_DIR/state-blob.schema.json"

if [ $# -lt 1 ]; then
    echo "Usage: $0 <blob.json>"
    exit 1
fi

if ! command -v ajv &> /dev/null; then
    echo "Error: ajv-cli not found. Install with: npm install -g ajv-cli"
    exit 1
fi

ajv validate -s "$SCHEMA" -d "$1"
echo "✓ Valid state blob"
