#!/bin/bash

# Frontend Build Script for BuildScale AI
# This script builds SDK, admin, and web frontends in place

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR/frontend/sdk"
ADMIN_DIR="$SCRIPT_DIR/frontend/admin"
WEB_DIR="$SCRIPT_DIR/frontend/web"

echo -e "${YELLOW}=== BuildScale AI Frontend Build Script ===${NC}\n"

# Build SDK (must be built first as it's a dependency)
echo -e "${YELLOW}[1/3] Building SDK...${NC}"
cd "$SDK_DIR"
if pnpm build; then
    echo -e "${GREEN}✓ SDK built successfully${NC}"
    echo -e "  Location: ${GREEN}$SDK_DIR/dist${NC}\n"
else
    echo -e "${RED}✗ SDK build failed${NC}"
    exit 1
fi

# Build Admin Frontend
echo -e "${YELLOW}[2/3] Building Admin Frontend...${NC}"
cd "$ADMIN_DIR"
if pnpm build; then
    echo -e "${GREEN}✓ Admin frontend built successfully${NC}"
    echo -e "  Location: ${GREEN}$ADMIN_DIR/dist${NC}\n"
else
    echo -e "${RED}✗ Admin frontend build failed${NC}"
    exit 1
fi

# Build Web Frontend
echo -e "${YELLOW}[3/3] Building Web Frontend...${NC}"
cd "$WEB_DIR"
if pnpm build; then
    echo -e "${GREEN}✓ Web frontend built successfully${NC}"
    echo -e "  Location: ${GREEN}$WEB_DIR/dist${NC}\n"
else
    echo -e "${RED}✗ Web frontend build failed${NC}"
    exit 1
fi

# Summary
echo -e "${GREEN}=== Build Complete ===${NC}"
echo -e "SDK:             ${GREEN}$SDK_DIR/dist${NC}"
echo -e "Admin frontend:  ${GREEN}$ADMIN_DIR/dist${NC}"
echo -e "Web frontend:    ${GREEN}$WEB_DIR/dist${NC}"
echo ""
echo -e "${YELLOW}Note: Backend .env is configured to serve from these directories${NC}"
echo -e "${YELLOW}Next steps:${NC}"
echo -e "1. Start the backend server: ${GREEN}cd backend && cargo run${NC}"
echo -e "2. Admin: ${GREEN}http://localhost:3000/admin${NC}"
echo -e "3. Web:   ${GREEN}http://localhost:3000/${NC}"
