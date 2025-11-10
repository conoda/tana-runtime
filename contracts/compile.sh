#!/usr/bin/env bash
#
# Pre-compile TypeScript contracts to JavaScript for faster execution
#
# Usage:
#   ./compile.sh              # Compile all contracts
#   ./compile.sh test         # Compile specific contract
#   ./compile.sh test get     # Compile specific method
#

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

compile_file() {
    local ts_file=$1
    local js_file="${ts_file%.ts}.js"

    echo -e "${BLUE}Compiling:${NC} $ts_file → $js_file"

    # Use bun to transpile TypeScript to JavaScript
    # --external tells bun to leave tana/* imports as-is
    bun build "$ts_file" \
        --outfile "$js_file" \
        --target=browser \
        --format=esm \
        --external="tana/*"

    echo -e "${GREEN}✓ Compiled:${NC} $js_file"
}

compile_contract() {
    local contract_id=$1
    local method=$2

    if [ -n "$method" ]; then
        # Compile specific method
        local ts_file="${contract_id}/${method}.ts"
        if [ -f "$ts_file" ]; then
            compile_file "$ts_file"
        else
            echo -e "${YELLOW}Warning:${NC} $ts_file not found"
        fi
    else
        # Compile all methods in contract
        for ts_file in "${contract_id}"/*.ts; do
            if [ -f "$ts_file" ]; then
                compile_file "$ts_file"
            fi
        done
    fi
}

main() {
    echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║   Tana Contract Pre-compiler          ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════╝${NC}\n"

    # Check if bun is installed
    if ! command -v bun &> /dev/null; then
        echo -e "${YELLOW}Error: bun is required but not installed${NC}"
        echo "Install bun: https://bun.sh"
        exit 1
    fi

    if [ $# -eq 0 ]; then
        # Compile all contracts
        for contract_dir in */; do
            if [ -d "$contract_dir" ]; then
                contract_id="${contract_dir%/}"
                echo -e "\n${BLUE}Contract:${NC} $contract_id"
                compile_contract "$contract_id"
            fi
        done
    elif [ $# -eq 1 ]; then
        # Compile specific contract
        compile_contract "$1"
    else
        # Compile specific contract method
        compile_contract "$1" "$2"
    fi

    echo -e "\n${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   Compilation Complete!                ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
}

main "$@"
