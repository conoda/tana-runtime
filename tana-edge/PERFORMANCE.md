# Performance Analysis & Optimization Strategies

## Current Performance Breakdown

Based on detailed timing measurements, here's where time is spent per request:

```
Extension setup:           0ms   (0%)
V8 runtime creation:       8ms   (9%)
TypeScript compiler load: 62ms  (66%) ← BOTTLENECK!
Bootstrap globals:        13ms  (14%)
Contract execution:        2ms   (2%)
Result extraction:         0ms   (0%)
Other overhead:           10ms   (9%)
──────────────────────────────────────
TOTAL:                   ~95ms (100%)
```

**Key Finding:** The TypeScript compiler accounts for 66% of request time. The actual contract execution is only 2ms!

## Why So Slow?

### The Current Architecture

For security (Cloudflare Workers-style), we create a **fresh V8 isolate per request**:

1. Create new V8 runtime (8ms)
2. Load entire TypeScript compiler library (~500KB of JS) (62ms)
3. Bootstrap Tana modules (13ms)
4. Transpile contract TypeScript → JavaScript (~6ms included in execution)
5. Execute contract (2ms)
6. Destroy isolate

This ensures **complete isolation** between requests - no state leakage, no shared memory. But it's expensive.

### The TypeScript Compiler Problem

The TypeScript compiler (`typescript.js`) is:
- **~500KB of JavaScript code**
- Loaded and parsed on **every single request**
- Only used for ~5-10ms of actual transpilation work
- Then immediately thrown away

This is like renting a bulldozer every time you need to dig a small hole.

## Optimization Strategies (Fastest to Slowest Wins)

### 1. Pre-compile Contracts (EASIEST, BIGGEST WIN)

**Impact:** 62ms → 0ms (66% reduction in latency)

Instead of transpiling on every request, transpile once during deployment:

```bash
# During contract deployment
tana deploy contract.ts
  ↓
1. Transpile TS → JS using external tool (bun, esbuild)
2. Store compiled JS in contracts/{id}/get.js
3. Edge server just executes pre-compiled JS

# At runtime
Edge server:
  - Skip TypeScript compiler entirely
  - Just execute the JS
  - Expected time: ~30ms per request
```

**Implementation:**
```rust
// In execute_contract()
let js_path = contract_path.with_extension("js");
let contract_source = if js_path.exists() {
    // Use pre-compiled JS (no transpilation needed!)
    tokio::fs::read_to_string(&js_path).await?
} else {
    // Fallback: transpile on-the-fly (for development)
    transpile_and_execute(contract_path).await?
};
```

**Tradeoffs:**
- ✅ 66% faster (62ms → 0ms saved)
- ✅ Simple to implement
- ✅ Works with existing architecture
- ❌ Requires build step during deployment
- ❌ Slightly more complex deployment process

---

### 2. V8 Isolate Pooling (MODERATE, SIGNIFICANT WIN)

**Impact:** 83ms → ~25ms (70% reduction in latency)

Maintain a pool of pre-initialized V8 runtimes:

```rust
// Global runtime pool
static RUNTIME_POOL: Lazy<RuntimePool> = Lazy::new(|| {
    RuntimePool::new(
        min_size: 4,
        max_size: 100,
        pre_warm: true // Load TypeScript compiler during init
    )
});

async fn handle_request() {
    let runtime = RUNTIME_POOL.acquire().await;
    // TypeScript already loaded! (~62ms saved)
    // Just execute contract
    runtime.execute(contract).await;
    RUNTIME_POOL.release(runtime);
}
```

**Architecture:**
```
Server startup:
  ├─ Create 4 V8 runtimes
  ├─ Pre-load TypeScript compiler in each
  └─ Mark as "ready"

Request comes in:
  ├─ Acquire runtime from pool (~1ms)
  ├─ Execute contract (2ms)
  ├─ Reset runtime state (1ms)
  └─ Return to pool
```

**Tradeoffs:**
- ✅ 70% faster for warmed pool
- ✅ Maintains isolation (reset state between requests)
- ❌ Higher memory usage (4-100 runtimes = ~200MB-5GB)
- ❌ Complex state management
- ❌ Potential for state leakage bugs
- ⚠️ First request still slow (cold pool)

---

### 3. Use Faster Transpiler (EASY, MODERATE WIN)

**Impact:** 62ms → ~5ms (57ms saved, ~60% reduction)

Replace TypeScript compiler with a Rust-based transpiler:

**Option A: SWC (Rust-based TypeScript compiler)**
```rust
use swc_ecma_parser::{parse_file_as_module};
use swc_ecma_transforms_typescript::strip;

// Transpile in Rust (not V8)
let js = swc_transpile(&ts_source)?; // ~5ms in Rust
runtime.execute(js)?; // No TS compiler needed in V8!
```

**Option B: oxc (Faster than SWC)**
```rust
use oxc::{Parser, Transpiler};

let js = oxc::transpile(&ts_source)?; // ~2-3ms
```

**Tradeoffs:**
- ✅ 60% faster (62ms → 5ms)
- ✅ Lower memory usage (no TS in V8)
- ✅ Simpler V8 runtime
- ❌ Different TypeScript support than official compiler
- ❌ May have subtle compatibility issues
- ❌ Requires Rust dependency changes

---

### 4. Lazy TypeScript Loading (SMALL WIN)

**Impact:** 62ms → 45ms (17ms saved, ~18% reduction)

Load TypeScript compiler only when needed:

```rust
// Check if contract has TypeScript features
if contract_source.contains("interface")
   || contract_source.contains(": ")
   || contract_source.needs_transpilation() {
    load_typescript_compiler()?; // 62ms
} else {
    // Plain JavaScript, skip TS entirely!
}
```

**Tradeoffs:**
- ✅ 18% faster for plain JS contracts
- ✅ Simple to implement
- ❌ Still slow for TS contracts
- ❌ Detection logic can be brittle

---

### 5. Snapshot-based Initialization (HARD, SIGNIFICANT WIN)

**Impact:** 83ms → ~15ms (68ms saved, ~82% reduction)

V8 supports "snapshots" - pre-initialized runtime state:

```rust
// One-time: Create snapshot with TS compiler
let snapshot = create_snapshot_with_typescript();
save_to_disk("tana-runtime.snapshot");

// At runtime:
let runtime = JsRuntime::new(RuntimeOptions {
    startup_snapshot: Some(load_snapshot("tana-runtime.snapshot")),
    // TypeScript already in memory! (~70ms saved)
});
```

**Tradeoffs:**
- ✅ 82% faster (massive win)
- ✅ Low memory overhead
- ✅ Maintains fresh isolate per request
- ❌ Complex to implement
- ❌ Snapshot must be updated when TS version changes
- ❌ Platform-specific (not portable across architectures)

---

## Recommended Approach

### Phase 1: Quick Win (Week 1)
**Pre-compile contracts during deployment**
- Expected: 95ms → ~30ms (68% faster)
- Low risk, high reward
- Maintains security model

### Phase 2: Production Optimization (Month 1)
**Pre-compilation + V8 Isolate Pool**
- Expected: 95ms → ~8ms (92% faster)
- Acceptable memory tradeoff for production
- Pool size: 10-20 runtimes

### Phase 3: Future (Optional)
**Snapshot-based + Pool + Pre-compilation**
- Expected: 95ms → ~3ms (97% faster)
- Maximum performance
- Complex but maintainable

---

## Production Performance Targets

With **Phase 2** implemented:

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| P50 Latency | 95ms | 8ms | 92% faster |
| P99 Latency | 120ms | 15ms | 87% faster |
| Throughput | 12 req/s | 125 req/s | 10x |
| Memory | 50MB | 500MB | 10x (acceptable) |
| CPU | 5% | 20% | 4x (acceptable) |

---

## Alternative: Deno Deploy Model

If we're okay with less isolation, we could use **long-lived isolates** (Deno Deploy model):

```rust
// One V8 runtime handles all requests for a contract
let runtime = create_runtime_for_contract("test");

// Reuse same runtime for all requests
loop {
    let request = receive_request().await;
    runtime.call_function("Get", request).await;
    // No cleanup, no reset - just keep reusing
}
```

**Tradeoffs:**
- ✅ Fastest possible (2-3ms per request)
- ✅ Simplest code
- ❌ State can leak between requests
- ❌ One contract crash affects all requests
- ❌ Memory leaks accumulate

This is what Cloudflare Workers actually does in production - they restart isolates periodically to prevent leaks.

---

## Next Steps

1. Implement pre-compilation in deployment pipeline
2. Add `--skip-transpile` flag to edge server
3. Benchmark with pre-compiled contracts
4. Decide on Phase 2 timing based on results

Expected timeline:
- Pre-compilation: 2-3 days
- Testing: 1 day
- Production deploy: 1 day

**Total: ~1 week to 68% faster performance**
