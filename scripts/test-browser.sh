#!/bin/bash
# Browser integration tests with automatic server management and smart change detection.
#
# Usage:
#   ./scripts/test-browser.sh           # Auto-detect which apps changed (for pre-commit)
#   ./scripts/test-browser.sh --all     # Test all apps (CI mode)
#   ./scripts/test-browser.sh pdfjoin   # Test specific app(s)
#   ./scripts/test-browser.sh pdfjoin docsign  # Test multiple specific apps
#
# The script automatically detects which apps have staged changes and only tests those.
# If shared code (crates/shared-*, scripts/*, etc.) changed, all apps are tested.

set -e
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Track PIDs for cleanup
AGENTPDF_PID=""
DOCSIGN_PID=""
PDFJOIN_PID=""

# Which apps to test
RUN_AGENTPDF=false
RUN_DOCSIGN=false
RUN_PDFJOIN=false

cleanup() {
    echo -e "\n${YELLOW}Cleaning up servers...${NC}"
    [[ -n "$AGENTPDF_PID" ]] && kill -0 "$AGENTPDF_PID" 2>/dev/null && kill "$AGENTPDF_PID" 2>/dev/null && echo "  Stopped agentpdf (PID $AGENTPDF_PID)"
    [[ -n "$DOCSIGN_PID" ]] && kill -0 "$DOCSIGN_PID" 2>/dev/null && kill "$DOCSIGN_PID" 2>/dev/null && echo "  Stopped docsign (PID $DOCSIGN_PID)"
    [[ -n "$PDFJOIN_PID" ]] && kill -0 "$PDFJOIN_PID" 2>/dev/null && kill "$PDFJOIN_PID" 2>/dev/null && echo "  Stopped pdfjoin (PID $PDFJOIN_PID)"
    # Kill any orphaned trunk processes
    lsof -ti:8080 2>/dev/null | xargs kill 2>/dev/null || true
    lsof -ti:8081 2>/dev/null | xargs kill 2>/dev/null || true
    lsof -ti:8082 2>/dev/null | xargs kill 2>/dev/null || true
}

trap cleanup EXIT

check_port() {
    local port=$1
    local name=$2
    if lsof -i:$port >/dev/null 2>&1; then
        echo -e "${YELLOW}Port $port in use (needed for $name), killing...${NC}"
        lsof -ti:$port | xargs kill -9 2>/dev/null || true
        sleep 1
        if lsof -i:$port >/dev/null 2>&1; then
            echo -e "${RED}Error: Could not free port $port${NC}"
            exit 1
        fi
        echo -e "  ${GREEN}✓${NC} Port $port freed"
    fi
}

wait_for_server() {
    local url=$1
    local name=$2
    local max_attempts=60
    local attempt=0

    echo -n "  Waiting for $name to be ready"
    while [ $attempt -lt $max_attempts ]; do
        if curl -s "$url" >/dev/null 2>&1; then
            echo -e " ${GREEN}✓${NC}"
            return 0
        fi
        echo -n "."
        sleep 1
        ((attempt++))
    done

    echo -e " ${RED}TIMEOUT${NC}"
    return 1
}

# ============================================================================
# CHANGE DETECTION
# ============================================================================

detect_apps_to_test() {
    # If explicit apps provided as arguments, use those
    if [[ $# -gt 0 ]]; then
        for arg in "$@"; do
            case "$arg" in
                --all)
                    RUN_AGENTPDF=true
                    RUN_DOCSIGN=true
                    RUN_PDFJOIN=true
                    ;;
                agentpdf) RUN_AGENTPDF=true ;;
                docsign) RUN_DOCSIGN=true ;;
                pdfjoin) RUN_PDFJOIN=true ;;
                *)
                    echo -e "${RED}Unknown app: $arg${NC}"
                    echo "Valid options: agentpdf, docsign, pdfjoin, --all"
                    exit 1
                    ;;
            esac
        done
        return 0
    fi

    # Auto-detect from staged git changes
    local detection_result
    detection_result=$("$SCRIPT_DIR/detect-changed-apps.sh" 2>/dev/null) || true

    case "$detection_result" in
        all)
            RUN_AGENTPDF=true
            RUN_DOCSIGN=true
            RUN_PDFJOIN=true
            ;;
        none|"")
            # No browser tests needed
            return 1
            ;;
        *)
            # Parse space-separated app list
            for app in $detection_result; do
                case "$app" in
                    agentpdf) RUN_AGENTPDF=true ;;
                    docsign) RUN_DOCSIGN=true ;;
                    pdfjoin) RUN_PDFJOIN=true ;;
                esac
            done
            ;;
    esac

    return 0
}

# ============================================================================
# MAIN
# ============================================================================

echo -e "${YELLOW}🌐 Browser Integration Tests${NC}"
echo "================================"

# Detect which apps to test
echo -e "\n${YELLOW}Detecting changes...${NC}"
if ! detect_apps_to_test "$@"; then
    echo -e "  ${CYAN}No app changes detected - skipping browser tests${NC}"
    echo -e "\n${GREEN}✅ No browser tests needed${NC}"
    exit 0
fi

# Show what we're testing
apps_list=""
[[ "$RUN_AGENTPDF" == "true" ]] && apps_list="$apps_list agentpdf"
[[ "$RUN_DOCSIGN" == "true" ]] && apps_list="$apps_list docsign"
[[ "$RUN_PDFJOIN" == "true" ]] && apps_list="$apps_list pdfjoin"
echo -e "  Testing:${CYAN}$apps_list${NC}"

# Check and free ports for apps we're testing
echo -e "\n${YELLOW}Checking ports...${NC}"
[[ "$RUN_AGENTPDF" == "true" ]] && check_port 8080 "agentpdf"
[[ "$RUN_DOCSIGN" == "true" ]] && check_port 8081 "docsign"
[[ "$RUN_PDFJOIN" == "true" ]] && check_port 8082 "pdfjoin"
echo -e "  ${GREEN}✓${NC} Required ports available"

# Start servers for apps we're testing
echo -e "\n${YELLOW}Starting servers...${NC}"

if [[ "$RUN_AGENTPDF" == "true" ]]; then
    cd "$PROJECT_ROOT/apps/agentpdf-web"
    trunk serve --port 8080 > /tmp/agentpdf-trunk.log 2>&1 &
    AGENTPDF_PID=$!
    echo "  agentpdf on :8080 (PID $AGENTPDF_PID)"
fi

if [[ "$RUN_DOCSIGN" == "true" ]]; then
    cd "$PROJECT_ROOT/apps/docsign-web"
    trunk serve --port 8081 > /tmp/docsign-trunk.log 2>&1 &
    DOCSIGN_PID=$!
    echo "  docsign on :8081 (PID $DOCSIGN_PID)"
fi

if [[ "$RUN_PDFJOIN" == "true" ]]; then
    cd "$PROJECT_ROOT/apps/pdfjoin-web"
    trunk serve --port 8082 > /tmp/pdfjoin-trunk.log 2>&1 &
    PDFJOIN_PID=$!
    echo "  pdfjoin on :8082 (PID $PDFJOIN_PID)"
fi

# Wait for servers
echo -e "\n${YELLOW}Waiting for servers...${NC}"
[[ "$RUN_AGENTPDF" == "true" ]] && { wait_for_server "http://127.0.0.1:8080" "agentpdf" || { echo -e "${RED}Failed to start agentpdf. Check /tmp/agentpdf-trunk.log${NC}"; exit 1; }; }
[[ "$RUN_DOCSIGN" == "true" ]] && { wait_for_server "http://127.0.0.1:8081" "docsign" || { echo -e "${RED}Failed to start docsign. Check /tmp/docsign-trunk.log${NC}"; exit 1; }; }
[[ "$RUN_PDFJOIN" == "true" ]] && { wait_for_server "http://127.0.0.1:8082" "pdfjoin" || { echo -e "${RED}Failed to start pdfjoin. Check /tmp/pdfjoin-trunk.log${NC}"; exit 1; }; }

# Run tests
echo -e "\n${YELLOW}Running browser tests...${NC}"
cd "$PROJECT_ROOT"

TEST_FAILED=0

# Determine test runner
if command -v cargo-nextest &> /dev/null; then
    TEST_CMD="cargo nextest run"
else
    TEST_CMD="cargo test"
fi

if [[ "$RUN_DOCSIGN" == "true" ]]; then
    echo -e "\n  ${YELLOW}DocSign tests:${NC}"
    if $TEST_CMD -p benchmark-harness --test browser_docsign 2>&1 | tee /tmp/docsign-tests.log; then
        echo -e "  ${GREEN}✓ DocSign tests passed${NC}"
    else
        echo -e "  ${RED}✗ DocSign tests failed${NC}"
        TEST_FAILED=1
    fi
fi

if [[ "$RUN_AGENTPDF" == "true" ]]; then
    echo -e "\n  ${YELLOW}AgentPDF tests:${NC}"
    if $TEST_CMD -p benchmark-harness --test browser_agentpdf 2>&1 | tee /tmp/agentpdf-tests.log; then
        echo -e "  ${GREEN}✓ AgentPDF tests passed${NC}"
    else
        echo -e "  ${RED}✗ AgentPDF tests failed${NC}"
        TEST_FAILED=1
    fi
fi

if [[ "$RUN_PDFJOIN" == "true" ]]; then
    echo -e "\n  ${YELLOW}PDFJoin tests:${NC}"
    if $TEST_CMD -p benchmark-harness --test browser_pdfjoin 2>&1 | tee /tmp/pdfjoin-tests.log; then
        echo -e "  ${GREEN}✓ PDFJoin tests passed${NC}"
    else
        echo -e "  ${RED}✗ PDFJoin tests failed${NC}"
        TEST_FAILED=1
    fi
fi

# Summary
echo -e "\n================================"
if [ $TEST_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All browser tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some browser tests failed${NC}"
    echo "  Check logs in /tmp/*-tests.log"
    exit 1
fi
