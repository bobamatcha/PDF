#!/bin/bash
# Browser integration tests with automatic server management
# Usage: ./scripts/test-browser.sh [--quick]
#
# Starts trunk servers for both apps, runs browser tests, then cleans up.
# Use --quick to skip the full test suite and only run browser tests.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track PIDs for cleanup
AGENTPDF_PID=""
DOCSIGN_PID=""

cleanup() {
    echo -e "\n${YELLOW}Cleaning up servers...${NC}"
    if [ -n "$AGENTPDF_PID" ] && kill -0 "$AGENTPDF_PID" 2>/dev/null; then
        kill "$AGENTPDF_PID" 2>/dev/null || true
        echo "  Stopped agentpdf server (PID $AGENTPDF_PID)"
    fi
    if [ -n "$DOCSIGN_PID" ] && kill -0 "$DOCSIGN_PID" 2>/dev/null; then
        kill "$DOCSIGN_PID" 2>/dev/null || true
        echo "  Stopped docsign server (PID $DOCSIGN_PID)"
    fi
    # Also kill any orphaned trunk processes on our ports
    lsof -ti:8080 | xargs kill 2>/dev/null || true
    lsof -ti:8081 | xargs kill 2>/dev/null || true
}

trap cleanup EXIT

check_port() {
    local port=$1
    local name=$2
    if lsof -i:$port >/dev/null 2>&1; then
        echo -e "${RED}Error: Port $port is already in use (needed for $name)${NC}"
        echo "  Kill the process with: lsof -ti:$port | xargs kill"
        exit 1
    fi
}

wait_for_server() {
    local url=$1
    local name=$2
    local max_attempts=60  # 60 seconds max wait
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

echo -e "${YELLOW}🌐 Browser Integration Tests${NC}"
echo "================================"

# Check ports are free
echo -e "\n${YELLOW}Checking ports...${NC}"
check_port 8080 "agentpdf"
check_port 8081 "docsign"
echo -e "  Ports 8080 and 8081 are ${GREEN}available${NC}"

# Start agentpdf server
echo -e "\n${YELLOW}Starting agentpdf server on :8080...${NC}"
cd "$PROJECT_ROOT/apps/agentpdf-web"
trunk serve --port 8080 > /tmp/agentpdf-trunk.log 2>&1 &
AGENTPDF_PID=$!
echo "  PID: $AGENTPDF_PID"

# Start docsign server
echo -e "\n${YELLOW}Starting docsign server on :8081...${NC}"
cd "$PROJECT_ROOT/apps/docsign-web"
trunk serve --port 8081 > /tmp/docsign-trunk.log 2>&1 &
DOCSIGN_PID=$!
echo "  PID: $DOCSIGN_PID"

# Wait for servers to be ready
echo -e "\n${YELLOW}Waiting for servers...${NC}"
if ! wait_for_server "http://127.0.0.1:8080" "agentpdf"; then
    echo -e "${RED}Failed to start agentpdf. Check /tmp/agentpdf-trunk.log${NC}"
    exit 1
fi
if ! wait_for_server "http://127.0.0.1:8081" "docsign"; then
    echo -e "${RED}Failed to start docsign. Check /tmp/docsign-trunk.log${NC}"
    exit 1
fi

# Run browser tests
echo -e "\n${YELLOW}Running browser tests...${NC}"
cd "$PROJECT_ROOT"

TEST_FAILED=0

# Run docsign browser tests
echo -e "\n  ${YELLOW}DocSign tests:${NC}"
if cargo test -p benchmark-harness --test browser_docsign -- --nocapture 2>&1 | tee /tmp/docsign-tests.log; then
    echo -e "  ${GREEN}✓ DocSign tests passed${NC}"
else
    echo -e "  ${RED}✗ DocSign tests failed${NC}"
    TEST_FAILED=1
fi

# Run agentpdf browser tests
echo -e "\n  ${YELLOW}AgentPDF tests:${NC}"
if cargo test -p benchmark-harness --test browser_agentpdf -- --nocapture 2>&1 | tee /tmp/agentpdf-tests.log; then
    echo -e "  ${GREEN}✓ AgentPDF tests passed${NC}"
else
    echo -e "  ${RED}✗ AgentPDF tests failed${NC}"
    TEST_FAILED=1
fi

# Summary
echo -e "\n================================"
if [ $TEST_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All browser tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some browser tests failed${NC}"
    echo "  Check logs at /tmp/docsign-tests.log and /tmp/agentpdf-tests.log"
    exit 1
fi
