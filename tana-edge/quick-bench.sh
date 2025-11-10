#!/bin/bash

echo "🔥 Running 20 sequential requests to measure performance..."
echo ""

total=0
count=20

for i in $(seq 1 $count); do
    time_ms=$(curl -s -o /dev/null -w "%{time_total}" http://127.0.0.1:8180/test 2>&1 | awk '{print $1 * 1000}')
    total=$(echo "$total + $time_ms" | bc)
    printf "Request %2d: %.0f ms\n" $i $time_ms
done

avg=$(echo "scale=1; $total / $count" | bc)
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Average: ${avg}ms per request"
rps=$(echo "scale=1; 1000 / $avg" | bc)
echo "Throughput: ${rps} req/s"
echo ""
echo "Check server logs for detailed [METRICS] output!"
