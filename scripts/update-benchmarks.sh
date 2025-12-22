#!/bin/bash
# Update benchmark results in README.md
# Run from workspace root: ./scripts/update-benchmarks.sh
#
# Prerequisites:
#   - trunk installed (cargo install trunk)
#   - Chrome/Chromium installed
#   - All apps built with: trunk build --release (in each app dir)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
README="$WORKSPACE_ROOT/README.md"
BENCHMARK_MARKER_START="<!-- BENCHMARK_RESULTS_START -->"
BENCHMARK_MARKER_END="<!-- BENCHMARK_RESULTS_END -->"

cd "$WORKSPACE_ROOT"

echo "=== Collecting WASM Bundle Sizes ==="
AGENTPDF_WASM=$(ls -lh apps/agentpdf-web/www/dist/*.wasm 2>/dev/null | awk '{print $5}' | head -1 || echo "N/A")
DOCSIGN_WASM=$(ls -lh apps/docsign-web/www/dist/*.wasm 2>/dev/null | awk '{print $5}' | head -1 || echo "N/A")
PDFJOIN_WASM=$(ls -lh apps/pdfjoin-web/www/dist/*.wasm 2>/dev/null | awk '{print $5}' | head -1 || echo "N/A")

echo "  agentpdf-web: $AGENTPDF_WASM"
echo "  docsign-web:  $DOCSIGN_WASM"
echo "  pdfjoin-web:  $PDFJOIN_WASM"

# Check which servers are running
SERVERS_RUNNING=""
check_server() {
    local port=$1
    local name=$2
    if curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$port" 2>/dev/null | grep -q "200"; then
        SERVERS_RUNNING="$SERVERS_RUNNING $name"
        return 0
    fi
    return 1
}

echo ""
echo "=== Checking Local Servers ==="
check_server 8080 "agentpdf" && echo "  agentpdf-web: running on :8080" || echo "  agentpdf-web: not running"
check_server 8081 "docsign" && echo "  docsign-web:  running on :8081" || echo "  docsign-web:  not running"
check_server 8082 "pdfjoin" && echo "  pdfjoin-web:  running on :8082" || echo "  pdfjoin-web:  not running"

# Run benchmarks for available servers
RESULTS_JSON="$WORKSPACE_ROOT/.benchmark-results.json"
echo '{}' > "$RESULTS_JSON"

run_benchmark() {
    local name=$1
    local config=$2
    echo ""
    echo "=== Running $name Benchmarks ==="

    # Run benchmark and capture output
    OUTPUT=$(cargo run -p benchmark-harness --example run_benchmark -- "$config" 2>&1 || true)

    # Extract LCP p95 and CLS from output
    LCP_P95=$(echo "$OUTPUT" | grep -A5 "Homepage Load" | grep "LCP" | awk '{print $6}' | head -1 || echo "N/A")
    CLS=$(echo "$OUTPUT" | grep -A5 "Homepage Load" | grep "CLS" | awk '{print $6}' | head -1 || echo "N/A")

    echo "  LCP p95: ${LCP_P95}ms"
    echo "  CLS: $CLS"

    # Store results
    if [ "$LCP_P95" != "N/A" ]; then
        jq --arg name "$name" --arg lcp "$LCP_P95" --arg cls "$CLS" \
            '. + {($name): {"lcp_p95": $lcp, "cls": $cls}}' "$RESULTS_JSON" > "$RESULTS_JSON.tmp"
        mv "$RESULTS_JSON.tmp" "$RESULTS_JSON"
    fi
}

if echo "$SERVERS_RUNNING" | grep -q "agentpdf"; then
    run_benchmark "agentpdf" "crates/benchmark-harness/scenarios/agentpdf.toml"
fi

if echo "$SERVERS_RUNNING" | grep -q "pdfjoin"; then
    run_benchmark "pdfjoin" "crates/benchmark-harness/scenarios/pdfjoin.toml"
fi

# Generate benchmark markdown section
TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M UTC")

generate_benchmark_section() {
    cat << 'HEADER'
## Performance Benchmarks

> **Last updated**: TIMESTAMP_PLACEHOLDER
>
> Run `./scripts/update-benchmarks.sh` to refresh these results.

### WASM Bundle Sizes

| App | Bundle Size | Description |
|-----|-------------|-------------|
HEADER

    # Add bundle sizes
    echo "| **pdfjoin-web** | **$PDFJOIN_WASM** | PDF split/merge only |"
    echo "| docsign-web | $DOCSIGN_WASM | Document signing + crypto |"
    echo "| agentpdf-web | $AGENTPDF_WASM | Full Typst template engine |"

    cat << 'METRICS'

### Core Web Vitals

| App | LCP (p95) | CLS | Status |
|-----|-----------|-----|--------|
METRICS

    # Add metrics from JSON
    PDFJOIN_LCP=$(jq -r '.pdfjoin.lcp_p95 // "—"' "$RESULTS_JSON")
    PDFJOIN_CLS=$(jq -r '.pdfjoin.cls // "—"' "$RESULTS_JSON")
    AGENTPDF_LCP=$(jq -r '.agentpdf.lcp_p95 // "—"' "$RESULTS_JSON")
    AGENTPDF_CLS=$(jq -r '.agentpdf.cls // "—"' "$RESULTS_JSON")

    # Determine status based on thresholds (LCP < 500ms, CLS < 0.1)
    get_status() {
        local lcp=$1
        local cls=$2
        if [ "$lcp" = "—" ] || [ "$cls" = "—" ]; then
            echo "Not tested"
        elif [ "$(echo "$lcp < 500" | bc -l)" = "1" ] && [ "$(echo "$cls < 0.1" | bc -l)" = "1" ]; then
            echo "PASS"
        else
            echo "WARN"
        fi
    }

    PDFJOIN_STATUS=$(get_status "$PDFJOIN_LCP" "$PDFJOIN_CLS")
    AGENTPDF_STATUS=$(get_status "$AGENTPDF_LCP" "$AGENTPDF_CLS")

    if [ "$PDFJOIN_LCP" != "—" ]; then
        echo "| **pdfjoin-web** | **${PDFJOIN_LCP}ms** | ${PDFJOIN_CLS} | $PDFJOIN_STATUS |"
    else
        echo "| pdfjoin-web | — | — | Not tested |"
    fi

    if [ "$AGENTPDF_LCP" != "—" ]; then
        echo "| agentpdf-web | ${AGENTPDF_LCP}ms | ${AGENTPDF_CLS} | $AGENTPDF_STATUS |"
    else
        echo "| agentpdf-web | — | — | Not tested |"
    fi

    echo "| docsign-web | — | — | Not tested |"

    cat << 'THRESHOLDS'

**Thresholds**: LCP < 500ms (localhost), CLS < 0.1

### Running Benchmarks

```bash
# Build all apps first
cd apps/agentpdf-web && trunk build --release && cd ../..
cd apps/docsign-web && trunk build --release && cd ../..
cd apps/pdfjoin-web && trunk build --release && cd ../..

# Start servers (in separate terminals)
cd apps/agentpdf-web && trunk serve --port 8080
cd apps/docsign-web && trunk serve --port 8081
cd apps/pdfjoin-web && trunk serve --port 8082

# Update README with latest results
./scripts/update-benchmarks.sh

# Or run individual benchmarks manually
cargo run -p benchmark-harness --example run_benchmark -- crates/benchmark-harness/scenarios/pdfjoin.toml
cargo run -p benchmark-harness --example run_benchmark -- crates/benchmark-harness/scenarios/agentpdf.toml
```

THRESHOLDS
}

BENCHMARK_CONTENT=$(generate_benchmark_section | sed "s/TIMESTAMP_PLACEHOLDER/$TIMESTAMP/")

# Update README.md
echo ""
echo "=== Updating README.md ==="

if grep -q "$BENCHMARK_MARKER_START" "$README"; then
    # Replace existing benchmark section
    # Use awk to replace content between markers
    awk -v start="$BENCHMARK_MARKER_START" -v end="$BENCHMARK_MARKER_END" -v content="$BENCHMARK_CONTENT" '
        $0 ~ start { print; print content; skip=1; next }
        $0 ~ end { skip=0 }
        !skip { print }
    ' "$README" > "$README.tmp"
    mv "$README.tmp" "$README"
    echo "  Updated existing benchmark section"
else
    # Insert new benchmark section after the first heading block
    # Find line after "## Documentation" section ends (before "## Quick Start")
    awk -v start="$BENCHMARK_MARKER_START" -v end="$BENCHMARK_MARKER_END" -v content="$BENCHMARK_CONTENT" '
        /^## Quick Start/ && !inserted {
            print start
            print content
            print end
            print ""
            inserted=1
        }
        { print }
    ' "$README" > "$README.tmp"
    mv "$README.tmp" "$README"
    echo "  Inserted new benchmark section"
fi

# Cleanup
rm -f "$RESULTS_JSON"

echo ""
echo "=== Done ==="
echo "README.md updated with latest benchmark results"
