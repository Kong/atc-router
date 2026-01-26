#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CDEFS_FILE="$PROJECT_ROOT/lib/resty/router/cdefs.lua"

echo "Generating FFI C definitions with cbindgen..."

# luajit doesn't handle preprocessor defines
CBINDGEN_OUTPUT=$(cbindgen -l c | grep -v -F '#define')

TEMP_FILE=$(mktemp)

{
  # add everything up to and including the cdef call
  awk '/ffi\.cdef\(\[\[/{print; exit} {print}' "$CDEFS_FILE"

  printf "%s\n" "$CBINDGEN_OUTPUT"

  # add everything after the ending `]]`
  awk '/^\]\]\)/{found=1} found' "$CDEFS_FILE"
} > "$TEMP_FILE"

# Replace the original file
mv "$TEMP_FILE" "$CDEFS_FILE"

echo "Updated $CDEFS_FILE"
