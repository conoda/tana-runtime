# Tana Runtime

A lightweight experimental JavaScript/TypeScript runtime built on **deno_core** (Deno's V8 engine). It provides a sandboxed execution environment for TypeScript smart contracts, similar to Cloudflare Workers, designed for a ledger/blockchain system.

---

## Quick Start

```bash
# Run the Rust CLI runtime
cargo run

# Start the browser playground
cd playground && npm run dev
# Open http://localhost:4322/
```

---

## Overview

Tana Runtime creates a secure sandbox for executing TypeScript smart contracts with:

1. **V8 JavaScript Engine** via deno_core
2. **TypeScript Compiler** (typescript.js) - dynamically loads and transpiles
3. **Custom Module System** - Virtual modules like `tana:core`, `tana:data`, `tana:utils`
4. **Sandbox Isolation** - Hides Deno API, exposes only whitelisted functionality
5. **Dual Environment Support** - Same code runs in both CLI runtime and browser playground

---

## Implemented Features

### ✅ Storage API (`tana:data`)

**Status: FEATURE PARITY ACHIEVED** - Works identically in both environments

```typescript
import { data } from 'tana:data'

// Set values (staged, not committed yet)
await data.set('counter', 42)
await data.set('user', { name: 'Alice', balance: 1000 })

// Read values
const count = await data.get('counter')  // 42
const user = await data.get('user')      // { name: 'Alice', balance: 1000 }

// Pattern matching
await data.set('user:1:name', 'Bob')
await data.set('user:2:name', 'Charlie')
const userKeys = await data.keys('user:*')  // ['user:1:name', 'user:2:name']

// Atomic commit (all or nothing)
await data.commit()
```

**Implementation:**
- **Playground**: localStorage backend with full persistence
- **Rust Runtime**: In-memory HashMap (works but resets each run)
- **Planned**: Redis backend for production persistence

**Storage Limits:**
- Max key size: 256 bytes
- Max value size: 10 KB
- Max total size: 100 KB per contract
- Max keys: 1000

**Key Features:**
- Staging buffer with atomic commits
- Size validation before persistence
- JSON auto-serialization
- Glob pattern matching support

### ✅ Fetch API (`tana:utils`)

**Status: WORKING IN BOTH ENVIRONMENTS**

```typescript
import { fetch } from 'tana:utils'

// Whitelisted domains only
const data = await fetch('https://pokeapi.co/api/v2/pokemon/ditto')
console.log(data)
```

**Security:**
- Domain whitelist: `pokeapi.co`, `tana.dev`, `localhost`, etc.
- Rust: reqwest + tokio async runtime
- Playground: browser fetch with same whitelist

### ✅ Console API (`tana:core`)

**Status: WORKING IN BOTH ENVIRONMENTS**

```typescript
import { console } from 'tana:core'

console.log('Hello from Tana!')
console.error('Error message')

// Runtime version info
import { version } from 'tana:core'
console.log(version.tana)       // "0.1.0"
console.log(version.deno_core)  // "0.338"
console.log(version.v8)         // "13.2.281.5"
```

---

## Feature Parity Strategy

**Dual Environment Support:**

1. **Rust CLI Runtime** (`cargo run`) - Production-ready V8 sandbox
2. **Browser Playground** (Astro/Svelte web app) - Development/testing UI

**Synchronization Points:**
- `src/main.rs` - Rust ops definitions
- `playground/src/pages/sandbox.astro` - JavaScript module implementations
- `types/*.d.ts` - Shared TypeScript definitions
- `playground/src/components/Editor.svelte` - Monaco type definitions

**Rule:** If it's in the type definitions, it MUST work in BOTH environments.

See [FEATURE_PARITY.md](./FEATURE_PARITY.md) for detailed implementation strategy.

---

## Architecture

### Runtime Stack

```
┌─────────────────────────────────────┐
│   TypeScript Smart Contracts        │
├─────────────────────────────────────┤
│   Virtual Modules (tana:core, etc)  │
├─────────────────────────────────────┤
│   TypeScript Compiler (typescript.js)│
├─────────────────────────────────────┤
│   Sandboxed V8 Runtime (deno_core)  │
├─────────────────────────────────────┤
│   Rust Runtime / Browser Playground │
└─────────────────────────────────────┘
```

### Core Components

- **src/main.rs** - Main Rust runtime entry point, defines ops and bootstraps V8
- **src/lib.rs** - Library exports for WASM builds
- **build.rs** - Extracts version metadata at compile time
- **tana-globals.ts** - Bootstrap code that defines `globalThis.tana`
- **typescript.js** - Embedded TypeScript compiler (bundled)

### Execution Flow

1. `main.rs` bootstraps V8 + deno_core runtime
2. Loads `typescript.js` compiler into V8
3. Injects `tana-globals.ts` and hides `Deno` API
4. Registers virtual modules (`tana:core`, `tana:data`, etc.)
5. Reads user script (e.g., `example.ts`)
6. Transpiles TypeScript → JavaScript
7. Executes in sandbox with isolated state

---

## Ledger System Integration

Tana Runtime is part of a larger ledger system that combines TypeScript smart contracts, PostgreSQL persistence, and a sandboxed runtime environment.

### Data Model

The ledger tracks **multi-currency balances**, deposits, withdrawals, and transactions using:
- **Block batching** for efficient transaction processing
- **Account-based validation** with state hashes
- **Ed25519 signatures** for cryptographic verification
- **Smart contracts** written in TypeScript as deterministic state machines

Each contract is identified by a **code hash** and can be referenced by:
- Friendly alias URLs: `tana.cash/@user/tx`
- Full hash addresses: `tana.cash/@user/ab6bjk8hbvv6zzz…`

### Database Schema

Key tables supporting the ledger:

- **`accounts`**: Multi-currency balances, metadata, versioned state hashes
- **`transactions`**: Submitted transactions with contract hash references
- **`blocks`**: Ordered, batched transactions with state root hashes
- **`contracts`**: Code blobs and hashes of smart contracts
- **`account_locks`**: Ensures single pending modification per account

---

## File Structure

### Core Runtime Files

| File | Description |
|------|-------------|
| **src/main.rs** | Main entry point. Initializes `JsRuntime`, loads internal modules, defines ops |
| **src/lib.rs** | Library exports for WASM builds |
| **build.rs** | Build script extracting version info (Tana, Deno Core, V8) |
| **Cargo.toml** | Rust dependencies and package configuration |
| **typescript.js** | Embedded TypeScript compiler for `.ts` → `.js` transpilation |
| **tana-globals.ts** | Bootstrap defining `globalThis.tana` and hiding `Deno` |

### TypeScript & Type Definitions

| File | Description |
|------|-------------|
| **types/tana.d.ts** | Type declarations for `tana` and `tana:core` module |
| **types/tana-data.d.ts** | Type declarations for `tana:data` storage module |
| **types/tana-utils.d.ts** | Type declarations for `tana:utils` utilities |
| **tsconfig.json** | TypeScript config mapping `"tana:*"` paths to type definitions |

### Example & Test Files

| File | Description |
|------|-------------|
| **example.ts** | Example TypeScript program using `tana:core` |
| **counter-test.ts** | Simple counter demonstrating storage API |
| **test-storage.ts** | Comprehensive storage API tests |
| **test-fetch.ts** | Fetch API tests |
| **examples/counter-contract.ts** | Example smart contract |

### Documentation

| File | Description |
|------|-------------|
| **README.md** | This file - project overview and implementation status |
| **DATA_STORAGE.md** | Storage API design, implementation plan, examples |
| **FEATURE_PARITY.md** | Dual-environment strategy, API availability matrix |
| **STORAGE_IMPLEMENTATION.md** | Implementation status, backend comparison, roadmap |
| **STORAGE_QUICKSTART.md** | Quick start guide, API usage examples, test instructions |

---

## Current Status

### Working ✅

- V8 runtime bootstrapping
- TypeScript transpilation
- Module system (`tana:core`, `tana:data`, `tana:utils`)
- Storage API with staging + atomic commits
- Fetch API with domain whitelist
- Browser playground with Monaco editor
- Feature parity between CLI and playground
- JSON auto-serialization for storage
- Glob pattern matching for keys

### In Progress 🚧

- Redis backend for persistent storage (Rust runtime)
- Docker setup for database services
- Blockchain integration (`tana:blockchain` module)
- Data View tab in playground UI
- Gas costs and storage rent model

### Planned 📋

- PostgreSQL integration for ledger state
- Ed25519 signature verification
- Block batching and Merkle proofs
- Multi-currency account system
- Contract deployment and versioning
- API endpoints for network access

---

## Technical Stack

### Rust Dependencies

```toml
deno_core = "0.338"          # V8 runtime
deno_error = "0.5.7"         # Error handling
reqwest = "0.12"             # HTTP client
tokio = "1"                  # Async runtime
redis = "0.27"               # Database client (added, not yet used)
serde_json = "1.0"           # JSON handling
wasm-bindgen = "0.2"         # WASM support
```

### Web Playground Stack

- **Astro** - Web framework
- **Svelte** - UI components
- **Monaco Editor** - Code editor with TypeScript autocomplete
- **localStorage** - Storage backend for `tana:data`

---

## Example Smart Contract

```typescript
import { console } from 'tana:core'
import { data } from 'tana:data'

// Simple token transfer contract
async function transfer(from: string, to: string, amount: number) {
  // Read current balances
  const fromBalance = parseInt((await data.get(`balance:${from}`)) as string)
  const toBalance = parseInt((await data.get(`balance:${to}`)) as string)

  // Validate
  if (fromBalance < amount) {
    throw new Error('Insufficient balance')
  }

  // Update balances
  await data.set(`balance:${from}`, String(fromBalance - amount))
  await data.set(`balance:${to}`, String(toBalance + amount))

  // Commit atomically (both balances updated or neither)
  await data.commit()

  console.log(`Transferred ${amount} from ${from} to ${to}`)
}

// Initialize balances
await data.set('balance:alice', '1000')
await data.set('balance:bob', '500')
await data.commit()

// Execute transfer
await transfer('alice', 'bob', 200)

// Check new balances
console.log('Alice:', await data.get('balance:alice'))  // '800'
console.log('Bob:', await data.get('balance:bob'))      // '700'
```

---

## Testing

### CLI Runtime

```bash
# Run example script
cargo run

# Run specific test
cargo run counter-test.ts
cargo run test-storage.ts
```

### Browser Playground

```bash
cd playground
npm install
npm run dev
```

Open http://localhost:4322/ and use the Monaco editor to write and run TypeScript contracts.

**Inspect Storage:**
1. Open Browser DevTools (F12)
2. Go to **Application > Local Storage**
3. Look for keys starting with `tana:data:`

---

## Next Steps

1. **Redis Integration** - Add persistent storage backend to Rust runtime
2. **Docker Compose** - Setup Redis + PostgreSQL services
3. **Data View Tab** - Add UI to visualize storage in playground
4. **Blockchain Module** - Implement `tana:blockchain` API
5. **Gas Costs** - Add metering for storage operations
6. **Contract Deployment** - Support uploading and versioning contracts

---

## Notes

- `globalThis.Deno` is deleted to ensure true sandbox isolation
- The current module system will be replaced by a Rust `ModuleLoader`
- Storage limits prevent abuse (similar to Ethereum gas model)
- Same TypeScript code runs in CLI and browser playground
- Contracts are deterministic state machines identified by code hash

---