#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$PROJECT_DIR/target/release/sni-spoof"
ARG="${1:-}"

# Handle flags (--wizard, --preset, etc.)
if [[ "$ARG" == --* ]]; then
    # Pass flag directly to binary
    echo -e "${GREEN}Running: $BINARY $ARG${NC}"
    echo ""
    if [ "$EUID" -eq 0 ]; then
        RUST_LOG=${RUST_LOG:-warn} "$BINARY" "$ARG"
    else
        echo -e "${YELLOW}This requires root privileges. Enter your password if prompted.${NC}"
        RUST_LOG=${RUST_LOG:-warn} sudo "$BINARY" "$ARG"
    fi
    exit 0
fi

# Handle config file
CONFIG="${ARG:-$PROJECT_DIR/config.json}"

# Check if config file exists
if [ ! -f "$CONFIG" ]; then
    echo -e "${RED}Error: Config file not found: $CONFIG${NC}"
    echo -e "${YELLOW}Usage:${NC}"
    echo "  ./run.sh              # Run with default config.json"
    echo "  ./run.sh /path/config # Run with custom config"
    echo "  ./run.sh --wizard     # Interactive setup"
    echo "  ./run.sh --preset hcaptcha  # Use hCaptcha preset"
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
