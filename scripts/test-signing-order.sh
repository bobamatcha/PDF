#!/bin/bash
# Property-based tests for signing order invariance.
#
# These tests verify that different signing orderings achieve the same result.
# They are computationally intensive and NOT run by default in precommit.
#
# Usage:
#   ./scripts/test-signing-order.sh           # Run all property tests
#   ./scripts/test-signing-order.sh --quick   # Run only non-browser tests (faster)
#   ./scripts/test-signing-order.sh --browser # Run only browser-based tests
#   ./scripts/test-signing-order.sh --clean   # Just cleanup, no tests
#
# The script handles:
#   - Starting trunk serve for docsign-web
#   - Running the property tests with --ignored flag
#   - Cleaning up browser processes and temp files
#   - Killing orphaned servers

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

# Track server PID
DOCSIGN_PID=""

# Parse arguments
RUN_QUICK=false
RUN_BROWSER=false
CLEAN_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --quick) RUN_QUICK=true ;;
        --browser) RUN_BROWSER=true ;;
        --clean) CLEAN_ONLY=true ;;
        --help|-h)
            echo "Usage: $0 [--quick|--browser|--clean]"
            echo ""
            echo "Options:"
            echo "  --quick    Run only non-browser property tests (faster)"
            echo "  --browser  Run only browser-based property tests"
            echo "  --clean    Just cleanup, don't run tests"
            echo ""
            echo "Without options, runs all property tests."
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
    esac
done

# If neither quick nor browser specified, run both
if [[ "$RUN_QUICK" == "false" && "$RUN_BROWSER" == "false" && "$CLEAN_ONLY" == "false" ]]; then
    RUN_QUICK=true
    RUN_BROWSER=true
fi

cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"

    # Kill our server if running
    if [[ -n "$DOCSIGN_PID" ]] && kill -0 "$DOCSIGN_PID" 2>/dev/null; then
        kill "$DOCSIGN_PID" 2>/dev/null && echo "  Stopped docsign server (PID $DOCSIGN_PID)"
    fi

    # Kill any orphaned trunk processes on port 8081
    if lsof -ti:8081 >/dev/null 2>&1; then
        echo "  Killing orphaned processes on port 8081..."
        lsof -ti:8081 | xargs kill -9 2>/dev/null || true
    fi

    # Clean up Chrome temp directories from benchmark-harness
    echo "  Cleaning browser temp directories..."
    rm -rf /tmp/benchmark-harness-* 2>/dev/null || true

    # Clean up test logs
    rm -f /tmp/docsign-trunk.log /tmp/docsign-proptest.log 2>/dev/null || true

    # Clean up any orphaned chromium processes (from headless tests)
    pkill -f "chrome.*benchmark-harness" 2>/dev/null || true
    pkill -f "chromium.*benchmark-harness" 2>/dev/null || true

    echo -e "  ${GREEN}✓${NC} Cleanup complete"
}

trap cleanup EXIT

# Run cleanup if requested
if [[ "$CLEAN_ONLY" == "true" ]]; then
    echo -e "${YELLOW}Running cleanup only...${NC}"
    cleanup
    echo -e "\n${GREEN}✅ Cleanup complete${NC}"
    exit 0
fi

echo -e "${YELLOW}🧪 Property-based Signing Order Tests${NC}"
echo "======================================"

# Free port 8081 if needed
check_port() {
    if lsof -i:8081 >/dev/null 2>&1; then
        echo -e "${YELLOW}Port 8081 in use, killing...${NC}"
        lsof -ti:8081 | xargs kill -9 2>/dev/null || true
        sleep 1
        if lsof -i:8081 >/dev/null 2>&1; then
            echo -e "${RED}Error: Could not free port 8081${NC}"
            exit 1
        fi
        echo -e "  ${GREEN}✓${NC} Port 8081 freed"
    fi
}

wait_for_server() {
    local max_attempts=60
    local attempt=0

    echo -n "  Waiting for docsign server"
    while [ $attempt -lt $max_attempts ]; do
        if curl -s "http://127.0.0.1:8081" >/dev/null 2>&1; then
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

# Start server for browser tests
if [[ "$RUN_BROWSER" == "true" ]]; then
    echo -e "\n${YELLOW}Starting docsign server for browser tests...${NC}"
    check_port

    cd "$PROJECT_ROOT/apps/docsign-web"
    trunk serve --port 8081 > /tmp/docsign-trunk.log 2>&1 &
    DOCSIGN_PID=$!
    echo "  Server starting on :8081 (PID $DOCSIGN_PID)"

    if ! wait_for_server; then
        echo -e "${RED}Failed to start docsign server. Check /tmp/docsign-trunk.log${NC}"
        exit 1
    fi

    cd "$PROJECT_ROOT"
fi

# Run tests
echo -e "\n${YELLOW}Running property tests...${NC}"
TEST_FAILED=0

# Determine test runner
if command -v cargo-nextest &> /dev/null; then
    TEST_CMD="cargo nextest run"
else
    TEST_CMD="cargo test"
fi

if [[ "$RUN_QUICK" == "true" ]]; then
    echo -e "\n  ${CYAN}Non-browser property tests:${NC}"
    if $TEST_CMD -p benchmark-harness --test browser_docsign_proptest -- --ignored \
        --test proptest_signing_order_invariance \
        --test proptest_parallel_mode_no_blocking \
        --test proptest_all_signers_in_final_document \
        --test proptest_signing_determinism \
        --test proptest_signature_uniqueness \
        2>&1 | tee /tmp/docsign-proptest.log; then
        echo -e "  ${GREEN}✓ Non-browser property tests passed${NC}"
    else
        echo -e "  ${RED}✗ Non-browser property tests failed${NC}"
        TEST_FAILED=1
    fi
fi

if [[ "$RUN_BROWSER" == "true" ]]; then
    echo -e "\n  ${CYAN}Browser-based property tests:${NC}"
    if $TEST_CMD -p benchmark-harness --test browser_docsign_proptest -- --ignored \
        --test proptest_browser_signing_order_ui_consistency \
        --test proptest_browser_session_state_consistency \
        2>&1 | tee -a /tmp/docsign-proptest.log; then
        echo -e "  ${GREEN}✓ Browser property tests passed${NC}"
    else
        echo -e "  ${RED}✗ Browser property tests failed${NC}"
        TEST_FAILED=1
    fi
fi

# Summary
echo -e "\n======================================"
if [ $TEST_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All property tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some property tests failed${NC}"
    echo "  Check /tmp/docsign-proptest.log for details"
    exit 1
fi
