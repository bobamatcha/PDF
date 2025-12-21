#!/bin/bash
# WASM size check - fails if WASM is larger than threshold
# This prevents bloat regressions
#
# Dev builds are ~3MB due to debug symbols - that's expected
# Release builds should be <1MB after optimization
#
# Usage: ./check-wasm-size.sh [--dev]

set -e

if [ "$1" = "--dev" ]; then
    MAX_SIZE_KB=4000  # 4MB max for dev WASM (has debug symbols)
    MODE="dev"
else
    MAX_SIZE_KB=1000  # 1MB max for release WASM
    MODE="release"
fi

check_wasm() {
    local wasm_dir="$1"
    local name="$2"

    if [ ! -d "$wasm_dir" ]; then
        echo "SKIP: $name dist not found at $wasm_dir"
        return 0
    fi

    local wasm_file=$(ls "$wasm_dir"/*.wasm 2>/dev/null | head -1)
    if [ -z "$wasm_file" ]; then
        echo "SKIP: No WASM file in $wasm_dir"
        return 0
    fi

    local size_bytes=$(stat -f%z "$wasm_file" 2>/dev/null || stat -c%s "$wasm_file")
    local size_kb=$((size_bytes / 1024))

    echo "$name ($MODE): ${size_kb}KB (max: ${MAX_SIZE_KB}KB)"

    if [ "$size_kb" -gt "$MAX_SIZE_KB" ]; then
        echo "FAIL: $name WASM is ${size_kb}KB, exceeds ${MAX_SIZE_KB}KB limit!"
        echo "Check for unused dependencies like pdf-extract"
        return 1
    fi

    echo "PASS: $name within size limit"
    return 0
}

cd "$(dirname "$0")/.."

failed=0

check_wasm "apps/docsign-web/www/dist" "docsign-web" || failed=1
check_wasm "apps/agentpdf-web/www/dist" "agentpdf-web" || failed=1

exit $failed
