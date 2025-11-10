# tana-edge Performance Testing

## Metrics Logging

The server now logs performance metrics to stdout for every request:

```
[METRICS] method=GET contract=test status=200 duration=78ms
[METRICS] method=POST contract=test status=200 duration=95ms
```

**Metric Fields:**
- `method` - HTTP method (GET/POST)
- `contract` - Contract ID being executed
- `status` - HTTP status code returned
- `duration` - Total request time in milliseconds (includes V8 isolate creation, contract execution, and response generation)

## Benchmarking Script

Use `bench.sh` to run performance tests:

```bash
# Run full benchmark (1000 requests, concurrency 10)
./bench.sh

# Run quick test (100 requests, concurrency 5)
./bench.sh quick

# Test only GET requests
./bench.sh get

# Test only POST requests
./bench.sh post
```

**Requirements:**
- Server must be running on http://localhost:8180
- `contracts/test/get.ts` and `contracts/test/post.ts` must exist

**Benchmark Tools (optional):**
- `wrk` (recommended) - `brew install wrk`
- `apache-bench` (ab) - Built-in on macOS/Linux
- `hey` - `go install github.com/rakyll/hey@latest`

If no benchmark tools are installed, the script falls back to a simple curl-based test.

## Test Contract

The `contracts/test/get.ts` contract queries blockchain state for realistic performance testing:

- Reads block height, timestamp, hash
- Queries gas usage
- Attempts to read user balance
- Tests both synchronous (block context) and async (ledger API) operations

## Performance Expectations

With blockchain queries enabled:

- **Latency:** 75-85ms per request (includes V8 isolate creation + blockchain queries)
- **Throughput:** ~12-13 req/s (sequential)
- **Status:** All 200 OK responses indicate successful blockchain integration

These numbers represent the complete end-to-end time including:
1. V8 isolate creation
2. TypeScript transpilation
3. Contract execution
4. Blockchain context queries
5. HTTP ledger API calls
6. Response serialization

## Analyzing Performance

**View real-time metrics:**
```bash
./target/release/tana-edge | grep METRICS
```

**Calculate average response time:**
```bash
./target/release/tana-edge 2>&1 | grep METRICS | awk '{print $7}' | sed 's/duration=//' | sed 's/ms//' | awk '{sum+=$1; count++} END {print "Average:", sum/count, "ms"}'
```

**Monitor while benchmarking:**
```bash
# Terminal 1: Run server
./target/release/tana-edge

# Terminal 2: Run benchmark
./bench.sh quick

# Observe [METRICS] output in Terminal 1
```

## Future Improvements

To improve performance:
- [ ] V8 isolate pooling (reuse isolates across requests)
- [ ] Contract caching (avoid reloading from disk)
- [ ] Blockchain query caching (reduce ledger API calls)
- [ ] WebAssembly compilation (faster execution)
- [ ] Request batching (process multiple requests in single isolate)

Current architecture prioritizes security (fresh isolate per request) over raw performance.
