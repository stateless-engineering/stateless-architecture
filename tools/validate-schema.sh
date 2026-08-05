#!/usr/bin/env bash
# tools/validate-schema.sh — Validate a state blob against the architecture spec
#
# Usage: ./tools/validate-schema.sh <blob.json>
#
# Uses Ajv 2020-12 (bundles the draft the schema declares). Requires Node.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPEC_DIR="$SCRIPT_DIR/../spec"
SCHEMA="$SPEC_DIR/state-blob.schema.json"

if [ $# -lt 1 ]; then
    echo "Usage: $0 <blob.json>"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo "Error: node not found"
    exit 1
fi

DOC="$(realpath "$1")"

# Install ajv locally if absent (no-save keeps the repo clean).
if [ ! -d "$SCRIPT_DIR/../node_modules/ajv" ]; then
    npm install --no-save ajv@8 ajv-formats > /dev/null 2>&1
fi

node -e "
const Ajv2020 = require('ajv/dist/2020.js');
const addFormats = require('ajv-formats');
const ajv = new Ajv2020({ strict: false });
addFormats(ajv);
const schema = require('$SCHEMA');
const validate = ajv.compile(schema);
const doc = require('$DOC');
if (!validate(doc)) {
  console.error(JSON.stringify(validate.errors, null, 1));
  process.exit(1);
}
console.log('✓ Valid state blob');
"
