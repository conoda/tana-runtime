#!/usr/bin/env bash
#
# Performance benchmarking script for tana-edge
#
# Usage:
#   ./bench.sh              # Run all benchmarks
#   ./bench.sh get          # Run only GET benchmarks
#   ./bench.sh post         # Run only POST benchmarks
#   ./bench.sh quick        # Quick test (fewer requests)
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="http://127.0.0.1:8180"
CONTRACT_ID="test"
WARMUP_REQUESTS=10
BENCH_REQUESTS=1000
BENCH_CONCURRENCY=10

# Parse arguments
MODE=${1:-all}
if [ "$MODE" = "quick" ]; then
    BENCH_REQUESTS=100
    BENCH_CONCURRENCY=5
fi

# Check if server is running
check_server() {
    echo -e "${BLUE}Checking if tana-edge is running...${NC}"
    if ! curl -s -f "$BASE_URL/$CONTRACT_ID" > /dev/null 2>&1; then
        echo -e "${RED}Error: tana-edge server is not running or contract not found${NC}"
        echo "Please start the server and ensure contracts/$CONTRACT_ID/get.ts exists"
        exit 1
    fi
    echo -e "${GREEN}✓ Server is running${NC}\n"
}

# Check for benchmark tools
check_tools() {
    echo -e "${BLUE}Checking for benchmark tools...${NC}"

    HAS_AB=false
    HAS_WRK=false
    HAS_HEY=false

    if command -v ab &> /dev/null; then
        HAS_AB=true
        echo -e "${GREEN}✓ Apache Bench (ab) found${NC}"
    fi

    if command -v wrk &> /dev/null; then
        HAS_WRK=true
        echo -e "${GREEN}✓ wrk found${NC}"
    fi

    if command -v hey &> /dev/null; then
        HAS_HEY=true
        echo -e "${GREEN}✓ hey found${NC}"
    fi

    if [ "$HAS_AB" = false ] && [ "$HAS_WRK" = false ] && [ "$HAS_HEY" = false ]; then
        echo -e "${YELLOW}Warning: No benchmark tools found${NC}"
        echo "Install one of: apache-bench, wrk, or hey"
        echo ""
        echo "  macOS:  brew install wrk"
        echo "  Ubuntu: sudo apt-get install apache2-utils"
        echo "  or:     go install github.com/rakyll/hey@latest"
        echo ""
        echo "Falling back to basic curl test..."
        return 1
    fi
    echo ""
    return 0
}

# Warmup
warmup() {
    echo -e "${BLUE}Warming up with $WARMUP_REQUESTS requests...${NC}"
    for i in $(seq 1 $WARMUP_REQUESTS); do
        curl -s "$BASE_URL/$CONTRACT_ID" > /dev/null 2>&1
    done
    echo -e "${GREEN}✓ Warmup complete${NC}\n"
}

# Benchmark with Apache Bench
bench_ab() {
    local method=$1
    local url=$2
    local name=$3

    echo -e "${YELLOW}Running: $name${NC}"
    echo "  Tool: Apache Bench (ab)"
    echo "  Requests: $BENCH_REQUESTS"
    echo "  Concurrency: $BENCH_CONCURRENCY"
    echo ""

    if [ "$method" = "GET" ]; then
        ab -n "$BENCH_REQUESTS" -c "$BENCH_CONCURRENCY" -q "$url"
    else
        ab -n "$BENCH_REQUESTS" -c "$BENCH_CONCURRENCY" -q \
           -p /dev/stdin -T 'application/json' "$url" <<< '{"test":"data"}'
    fi
    echo ""
}

# Benchmark with wrk
bench_wrk() {
    local method=$1
    local url=$2
    local name=$3

    echo -e "${YELLOW}Running: $name${NC}"
    echo "  Tool: wrk"
    echo "  Duration: 10s"
    echo "  Connections: $BENCH_CONCURRENCY"
    echo "  Threads: 4"
    echo ""

    if [ "$method" = "GET" ]; then
        wrk -t4 -c"$BENCH_CONCURRENCY" -d10s "$url"
    else
        wrk -t4 -c"$BENCH_CONCURRENCY" -d10s \
            -s <(echo 'wrk.method = "POST"; wrk.body = "{\"test\":\"data\"}"; wrk.headers["Content-Type"] = "application/json"') \
            "$url"
    fi
    echo ""
}

# Benchmark with hey
bench_hey() {
    local method=$1
    local url=$2
    local name=$3

    echo -e "${YELLOW}Running: $name${NC}"
    echo "  Tool: hey"
    echo "  Requests: $BENCH_REQUESTS"
    echo "  Concurrency: $BENCH_CONCURRENCY"
    echo ""

    if [ "$method" = "GET" ]; then
        hey -n "$BENCH_REQUESTS" -c "$BENCH_CONCURRENCY" "$url"
    else
        hey -n "$BENCH_REQUESTS" -c "$BENCH_CONCURRENCY" \
            -m POST -T 'application/json' -d '{"test":"data"}' \
            "$url"
    fi
    echo ""
}

# Basic curl benchmark (fallback)
bench_curl() {
    local method=$1
    local url=$2
    local name=$3
    local count=100

    echo -e "${YELLOW}Running: $name (basic curl test)${NC}"
    echo "  Requests: $count (sequential)"
    echo ""

    local total_time=0
    local successful=0

    for i in $(seq 1 $count); do
        if [ "$method" = "GET" ]; then
            response_time=$(curl -s -w "%{time_total}" -o /dev/null "$url" 2>&1)
        else
            response_time=$(curl -s -w "%{time_total}" -o /dev/null \
                -X POST -H "Content-Type: application/json" \
                -d '{"test":"data"}' "$url" 2>&1)
        fi

        if [ $? -eq 0 ]; then
            successful=$((successful + 1))
            total_time=$(echo "$total_time + $response_time" | bc)
        fi

        # Progress indicator
        if [ $((i % 10)) -eq 0 ]; then
            echo -n "."
        fi
    done

    echo ""
    echo ""
    echo "Results:"
    echo "  Successful requests: $successful/$count"

    if [ "$successful" -gt 0 ]; then
        avg_time=$(echo "scale=4; $total_time / $successful" | bc)
        echo "  Average time: ${avg_time}s ($(echo "$avg_time * 1000" | bc)ms)"

        rps=$(echo "scale=2; $successful / $total_time" | bc)
        echo "  Requests/sec: $rps"
    fi
    echo ""
}

# Run benchmarks
run_benchmarks() {
    local test_type=$1

    if [ "$test_type" = "get" ] || [ "$test_type" = "all" ]; then
        echo -e "${BLUE}=== GET Request Benchmarks ===${NC}\n"

        if [ "$HAS_WRK" = true ]; then
            bench_wrk "GET" "$BASE_URL/$CONTRACT_ID" "GET /$CONTRACT_ID"
        elif [ "$HAS_AB" = true ]; then
            bench_ab "GET" "$BASE_URL/$CONTRACT_ID" "GET /$CONTRACT_ID"
        elif [ "$HAS_HEY" = true ]; then
            bench_hey "GET" "$BASE_URL/$CONTRACT_ID" "GET /$CONTRACT_ID"
        else
            bench_curl "GET" "$BASE_URL/$CONTRACT_ID" "GET /$CONTRACT_ID"
        fi
    fi

    if [ "$test_type" = "post" ] || [ "$test_type" = "all" ]; then
        echo -e "${BLUE}=== POST Request Benchmarks ===${NC}\n"

        if [ "$HAS_WRK" = true ]; then
            bench_wrk "POST" "$BASE_URL/$CONTRACT_ID" "POST /$CONTRACT_ID"
        elif [ "$HAS_AB" = true ]; then
            bench_ab "POST" "$BASE_URL/$CONTRACT_ID" "POST /$CONTRACT_ID"
        elif [ "$HAS_HEY" = true ]; then
            bench_hey "POST" "$BASE_URL/$CONTRACT_ID" "POST /$CONTRACT_ID"
        else
            bench_curl "POST" "$BASE_URL/$CONTRACT_ID" "POST /$CONTRACT_ID"
        fi
    fi
}

# Main
main() {
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   tana-edge Performance Benchmark     ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}\n"

    check_server
    if check_tools; then
        warmup
    fi

    case $MODE in
        get|GET)
            run_benchmarks "get"
            ;;
        post|POST)
            run_benchmarks "post"
            ;;
        quick)
            echo -e "${YELLOW}Running quick benchmark...${NC}\n"
            run_benchmarks "all"
            ;;
        all|*)
            run_benchmarks "all"
            ;;
    esac

    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║        Benchmark Complete!             ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BLUE}Check server logs for [METRICS] output${NC}"
}

main
