#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$PROJECT_DIR/target/release/sni-spoof"
CONFIG="${1:-$PROJECT_DIR/config.json}"

# Check if config file exists
if [ ! -f "$CONFIG" ]; then
    echo -e "${RED}Error: Config file not found: $CONFIG${NC}"
    exit 1
fi

# Build if binary doesn't exist
if [ ! -f "$BINARY" ]; then
    echo -e "${YELLOW}Binary not found. Building...${NC}"
    cd "$PROJECT_DIR"
    cargo build --release
fi

# Run with appropriate privileges
echo -e "${GREEN}Starting SNI Spoof Proxy...${NC}"
echo "Config: $CONFIG"
echo ""

if [ "$EUID" -eq 0 ]; then
    # Already root
    RUST_LOG=${RUST_LOG:-warn} "$BINARY" "$CONFIG"
else
    # Need sudo
    echo -e "${YELLOW}This requires root privileges. Enter your password if prompted.${NC}"
    RUST_LOG=${RUST_LOG:-warn} sudo "$BINARY" "$CONFIG"
fi
