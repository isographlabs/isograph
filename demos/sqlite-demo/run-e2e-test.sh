#!/usr/bin/env bash
# Run complete E2E test for Phase 1 SQL support
#
# This script:
# 1. Initializes test database
# 2. Compiles ISO literals → Substrait artifacts
# 3. Starts isograph-server
# 4. Runs Playwright E2E tests
# 5. Cleans up

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

echo "🚀 Phase 1 SQL Support E2E Test"
echo "================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Step 1: Initialize test database
echo -e "${YELLOW}Step 1: Initializing test database...${NC}"
if [ ! -f "${REPO_ROOT}/test-fixtures/databases/init-db.sh" ]; then
    echo -e "${RED}Error: Test database init script not found${NC}"
    exit 1
fi
"${REPO_ROOT}/test-fixtures/databases/init-db.sh"
echo -e "${GREEN}✓ Database initialized${NC}"
echo ""

# Step 2: Build compiler (if needed)
echo -e "${YELLOW}Step 2: Building Isograph compiler...${NC}"
cd "${REPO_ROOT}"
if [ ! -f "${REPO_ROOT}/target/debug/isograph_cli" ]; then
    echo "Compiler not found, building..."
    cargo build -p isograph_cli
fi
echo -e "${GREEN}✓ Compiler ready${NC}"
echo ""

# Step 3: Generate artifacts
echo -e "${YELLOW}Step 3: Generating Substrait artifacts...${NC}"
cd "${SCRIPT_DIR}"
"${REPO_ROOT}/target/debug/isograph_cli" --config ./isograph.config.json
echo -e "${GREEN}✓ Artifacts generated${NC}"
echo ""

# Verify artifacts were created
if [ ! -f "${SCRIPT_DIR}/src/components/__isograph/planets/HomePage/query_plan.bin" ]; then
    echo -e "${RED}Warning: query_plan.bin not found!${NC}"
    echo "Expected at: src/components/__isograph/planets/HomePage/query_plan.bin"
    echo "Phase 1 integration may not be complete."
fi

# Step 4: Start isograph-server in background
echo -e "${YELLOW}Step 4: Starting isograph-server...${NC}"
cd "${REPO_ROOT}/crates/isograph_server"
cargo build -p isograph_server 2>&1 | grep -E "(Finished|Compiling isograph_server)" || true

# Start server in background
cargo run -p isograph_server > /tmp/isograph-server.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"

# Wait for server to be ready
echo "Waiting for server to start..."
for i in {1..30}; do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Server started successfully${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}Error: Server failed to start${NC}"
        cat /tmp/isograph-server.log
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done
echo ""

# Step 5: Run Playwright tests
echo -e "${YELLOW}Step 5: Running E2E tests...${NC}"
cd "${SCRIPT_DIR}"

# Function to cleanup on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
        echo "Server stopped"
    fi
}
trap cleanup EXIT

# Run tests
npm run test:e2e

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}✓ E2E Test Suite Completed!${NC}"
echo -e "${GREEN}================================${NC}"
