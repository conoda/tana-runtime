# Feature Parity Between CLI Runtime and Playground

The Tana Playground must provide the **same developer experience** as the Rust CLI runtime, even though the implementations differ.

## Goal

Code written in the playground should work identically when run via `cargo run`, and vice versa.

## Current State

### Rust CLI Runtime (`src/main.rs`)
```rust
// Ops available via deno_core
#[op2]
fn op_sum(#[serde] nums: Vec<f64>) -> Result<f64, deno_error::JsErrorBox>

#[op2(fast)]
fn op_print_stderr(#[string] msg: String)

// Exposed to TypeScript as:
tanaModules["tana:core"] = {
  console: { log(), error() },
  version: { tana, deno_core, v8 }
}
```

### Playground (`playground/src/pages/sandbox.astro`)
```javascript
// Simulated in pure JavaScript
tanaModules['tana:core'] = {
  console: { log(), error() },
  version: { tana, deno_core, v8 }
}

tanaModules['tana:utils'] = {
  fetch()  // Whitelisted browser fetch
}
```

## Implementation Strategy

### 1. Shared API Specification

Maintain a single source of truth for the Tana TypeScript API in:
```
types/tana-core.d.ts  (Already exists - used by both!)
types/tana-utils.d.ts (To be created)
```

**Rule:** If it's in the type definitions, it must work in BOTH environments.

### 2. Rust → Playground Mapping

| Rust Implementation | Playground Implementation |
|---------------------|---------------------------|
| `deno_core::ops::op_*` | JavaScript function in `tanaModules` |
| `Deno.core.print()` | `document.getElementById('output').appendChild()` |
| `Deno.core.ops.op_print_stderr()` | `document.createElement('div').className = 'error'` |
| Future: Rust blockchain ops | Mock/simulated in JavaScript |

### 3. When Adding New Features

**Steps to maintain parity:**

#### A. Add to Rust Runtime
```rust
// src/main.rs or src/lib.rs
#[op2]
fn op_new_feature(#[string] input: String) -> String {
    // Actual implementation
}

// Expose in bootstrap
tanaModules["tana:utils"] = {
    newFeature(input) {
        return globalThis.__tanaCore.ops.op_new_feature(input);
    }
}
```

#### B. Mirror in Playground
```javascript
// playground/src/pages/sandbox.astro
tanaModules['tana:utils'] = {
    newFeature(input) {
        // Simulated/mocked implementation
        // Should behave the same as Rust version
    }
}
```

#### C. Update Type Definitions
```typescript
// types/tana-utils.d.ts (or appropriate file)
declare module 'tana:utils' {
    export function newFeature(input: string): string;
}
```

#### D. Update Monaco Types
```javascript
// playground/src/components/Editor.svelte
monaco.languages.typescript.typescriptDefaults.addExtraLib(
    `declare module 'tana:utils' {
        export function newFeature(input: string): string;
    }`,
    'ts:filename/tana-utils.d.ts'
);
```

## Example: Adding Blockchain Features

When you add blockchain operations:

### 1. Rust Side (Real Implementation)
```rust
// src/main.rs
#[op2]
fn op_create_transaction(
    #[string] from: String,
    #[string] to: String,
    amount: f64
) -> Result<String, deno_error::JsErrorBox> {
    // Actual blockchain logic
    let tx_id = blockchain::create_transaction(from, to, amount)?;
    Ok(tx_id)
}

// Expose in bootstrap
blockchain: {
    createTransaction(from, to, amount) {
        return globalThis.__tanaCore.ops.op_create_transaction(from, to, amount);
    }
}
```

### 2. Playground Side (Simulated)
```javascript
// playground/src/pages/sandbox.astro
tanaModules['tana:blockchain'] = {
    createTransaction(from, to, amount) {
        // Simulate transaction creation
        const txId = 'tx_' + Math.random().toString(36).slice(2);

        // Could fetch from real API when available:
        // return originalFetch('https://api.tana.dev/tx/create', {...})

        return Promise.resolve(txId);
    }
}
```

### 3. Shared Types
```typescript
// types/tana-blockchain.d.ts
declare module 'tana:blockchain' {
    /**
     * Create a new transaction
     * @param from - Sender address
     * @param to - Recipient address
     * @param amount - Amount to send
     * @returns Transaction ID
     */
    export function createTransaction(
        from: string,
        to: string,
        amount: number
    ): Promise<string>;
}
```

## Testing Parity

### Manual Testing Checklist

For each new feature, test:
- [ ] Works in CLI: `cargo run example.ts`
- [ ] Works in playground: Open in browser
- [ ] Same output in both environments
- [ ] Type definitions match implementation
- [ ] Monaco provides correct autocomplete

### Example Test Case

**Test file:** `examples/blockchain-test.ts`
```typescript
import { console } from 'tana:core'
import { createTransaction } from 'tana:blockchain'

const txId = await createTransaction('alice', 'bob', 100)
console.log('Transaction created:', txId)
```

**Expected behavior:**
- CLI: Creates real transaction, prints actual TX ID
- Playground: Simulates transaction, prints mock TX ID
- Both: Same TypeScript code, same console output format

## Fetch: ✅ PARITY ACHIEVED

**Status:** `fetch` now works in BOTH Rust runtime and playground!

### Implementation

#### Rust Implementation (src/main.rs)
```rust
// Whitelisted domains (same as playground)
const ALLOWED_DOMAINS: &[&str] = &[
    "pokeapi.co",
    "tana.dev",
    "api.tana.dev",
    "blockchain.tana.dev",
    "localhost",
    "127.0.0.1",
];

#[op2(async)]
#[string]
async fn op_fetch(#[string] url: String) -> Result<String, deno_error::JsErrorBox> {
    // Domain whitelist validation
    let parsed = reqwest::Url::parse(&url)?;
    let hostname = parsed.host_str()?;

    let is_allowed = ALLOWED_DOMAINS.iter().any(|domain| {
        hostname == *domain || hostname.ends_with(&format!(".{}", domain))
    });

    if !is_allowed {
        return Err(/* whitelist error */);
    }

    // Perform fetch
    let response = reqwest::get(&url).await?;
    let body = response.text().await?;
    Ok(body)
}
```

**Key requirements:**
- Tokio runtime with `flavor = "current_thread"` (deno_core requirement)
- Event loop must be driven with `runtime.run_event_loop()`
- Same domain whitelist as playground

## Current API Availability

```typescript
// tana:core - ✅ Available everywhere (console, version)
// tana:utils.fetch - ✅ Available everywhere (whitelisted domains)
// tana:data - ✅ Available everywhere!
//           - Playground: localStorage backend (persists in browser)
//           - Rust: In-memory HashMap (resets each run until Redis added)
// tana:blockchain - 🚧 Not yet implemented
```

**Verified feature parity:**
- ✅ `console.log()` / `console.error()` work identically
- ✅ `fetch()` from `tana:utils` works with same whitelist in both environments
- ✅ `data.set()`, `data.get()`, `data.commit()` work identically
- ✅ Same storage limits (256B keys, 10KB values, 100KB total)
- ✅ Same staging and atomic commit behavior
- ✅ Same TypeScript code runs in CLI and playground
- ✅ Same error messages for size limits and validation

Test both environments with:
```bash
# CLI
cargo run

# Playground
cd playground && npm run dev
```

## Checklist for New Features

- [ ] Implement in Rust (`src/main.rs` or `src/lib.rs`)
- [ ] Implement in playground (`playground/src/pages/sandbox.astro`)
- [ ] Add types to `types/*.d.ts`
- [ ] Add Monaco types to `playground/src/components/Editor.svelte`
- [ ] Test in CLI with `cargo run`
- [ ] Test in playground in browser
- [ ] Update `FEATURE_PARITY.md` (this file)
- [ ] Document any platform-specific limitations

## Files to Keep in Sync

1. **Rust runtime:** `src/main.rs` (ops and bootstrap)
2. **Playground runtime:** `playground/src/pages/sandbox.astro` (tanaModules)
3. **Type definitions:** `types/*.d.ts`
4. **Monaco types:** `playground/src/components/Editor.svelte` (addExtraLib calls)
5. **This document:** `FEATURE_PARITY.md`

When you modify one, check if others need updates!
