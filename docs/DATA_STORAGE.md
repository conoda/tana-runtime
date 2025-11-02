# Tana Data Storage Architecture

## Overview

Smart contracts need persistent key-value storage that works identically in both the Rust runtime (production) and browser playground (development/testing).

## Design Goals

1. **Simple KV API** - Easy for smart contract developers
2. **Size Limits** - Prevent abuse, similar to Ethereum gas model
3. **Atomic Commits** - State changes validated before persistence
4. **Feature Parity** - Same API in Rust (Redis) and Browser (localStorage)

## API Design

### Module: `tana:data`

```typescript
declare module 'tana:data' {
  /**
   * Contract data storage - Key-Value store
   */
  export const data: {
    /**
     * Set a value in the contract storage
     * @throws Error if key/value exceed size limits
     */
    set(key: string, value: string): Promise<void>;

    /**
     * Get a value from contract storage
     * @returns Value or null if not found
     */
    get(key: string): Promise<string | null>;

    /**
     * Delete a key from contract storage
     */
    delete(key: string): Promise<void>;

    /**
     * Check if key exists
     */
    has(key: string): Promise<boolean>;

    /**
     * List all keys matching pattern (glob-style)
     * @example keys('user:*') // Returns ['user:1', 'user:2', ...]
     */
    keys(pattern?: string): Promise<string[]>;

    /**
     * Get all entries as object
     */
    entries(): Promise<Record<string, string>>;

    /**
     * Clear all contract data (dev only)
     */
    clear(): Promise<void>;

    /**
     * Commit changes to blockchain
     * Validates size limits and atomically persists
     * @throws Error if validation fails
     */
    commit(): Promise<void>;
  };
}
```

## Storage Limits

```typescript
const LIMITS = {
  MAX_KEY_SIZE: 256,        // bytes
  MAX_VALUE_SIZE: 10_240,   // 10 KB
  MAX_TOTAL_SIZE: 102_400,  // 100 KB per contract
  MAX_KEYS: 1000            // max number of keys
};
```

## Implementation Strategy

### Phase 1: Playground (localStorage)

**Location:** `playground/src/pages/sandbox.astro`

```javascript
tanaModules['tana:data'] = {
  data: {
    // In-memory staging (not committed yet)
    _staging: new Map(),

    async set(key, value) {
      // Validate size limits
      if (key.length > 256) throw new Error('Key too large');
      if (value.length > 10240) throw new Error('Value too large');

      // Stage change
      this._staging.set(key, value);
    },

    async get(key) {
      // Check staging first, then localStorage
      if (this._staging.has(key)) {
        return this._staging.get(key);
      }
      return localStorage.getItem(`tana:data:${key}`);
    },

    async commit() {
      // Validate total size
      let totalSize = 0;
      for (const [key, value] of this._staging) {
        totalSize += key.length + value.length;
      }

      // Get existing storage size
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key?.startsWith('tana:data:')) {
          const value = localStorage.getItem(key);
          totalSize += key.length + (value?.length || 0);
        }
      }

      if (totalSize > 102400) {
        throw new Error('Storage limit exceeded: 100KB max');
      }

      // Commit staged changes
      for (const [key, value] of this._staging) {
        localStorage.setItem(`tana:data:${key}`, value);
      }

      this._staging.clear();
    }
  }
};
```

### Phase 2: Rust Runtime (Redis)

**Location:** `src/main.rs`

```rust
// Cargo.toml
redis = { version = "0.24", features = ["tokio-comp"] }

// src/main.rs
#[op2(async)]
async fn op_data_set(
    #[string] contract_id: String,
    #[string] key: String,
    #[string] value: String
) -> Result<(), deno_error::JsErrorBox> {
    // Validate sizes
    if key.len() > 256 {
        return Err(JsErrorBox::new("Error", "Key too large"));
    }
    if value.len() > 10_240 {
        return Err(JsErrorBox::new("Error", "Value too large"));
    }

    // Store in Redis with contract namespace
    let redis_key = format!("contract:{}:data:{}", contract_id, key);

    // TODO: Stage in transaction, commit atomically
    // For now, direct write
    redis_client.set(redis_key, value).await?;

    Ok(())
}
```

### Phase 3: Playground UI Enhancement

**New Tab:** Data View

```svelte
<!-- playground/src/components/Editor.svelte -->
<div class="output-panel">
  <div class="tabs">
    <button class:active={activeTab === 'output'}>Output</button>
    <button class:active={activeTab === 'data'}>Data</button>
  </div>

  {#if activeTab === 'output'}
    <iframe src="/sandbox" />
  {:else if activeTab === 'data'}
    <div class="data-view">
      <table>
        <thead>
          <tr>
            <th>Key</th>
            <th>Value</th>
            <th>Size</th>
          </tr>
        </thead>
        <tbody>
          {#each dataEntries as [key, value]}
            <tr>
              <td>{key}</td>
              <td>{value}</td>
              <td>{key.length + value.length} bytes</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <div class="storage-stats">
        Total: {totalSize} / 102,400 bytes ({percentage}%)
      </div>
    </div>
  {/if}
</div>
```

## Example Contract

```typescript
import { console } from 'tana:core'
import { data } from 'tana:data'

// Simple counter contract
const current = await data.get('counter')
const count = current ? parseInt(current) : 0

console.log('Current count:', count)

await data.set('counter', String(count + 1))
await data.set('lastUpdate', Date.now().toString())

// Commit changes to blockchain
await data.commit()

console.log('Counter incremented!')
```

## Roadmap

### Milestone 1: Basic KV Store
- [ ] Design `tana:data` API
- [ ] Implement localStorage backend in playground
- [ ] Add type definitions (`types/tana-data.d.ts`)
- [ ] Add Monaco autocomplete
- [ ] Basic size validation

### Milestone 2: Playground UI
- [ ] Add Data tab to output panel
- [ ] Real-time data view (shows KV pairs)
- [ ] Storage usage indicator
- [ ] Clear data button

### Milestone 3: Rust Runtime
- [ ] Redis integration
- [ ] Contract namespace isolation
- [ ] Atomic transactions (staging → commit)
- [ ] Size limit enforcement

### Milestone 4: Production Features
- [ ] Gas costs for storage operations
- [ ] Storage rent (like Ethereum)
- [ ] Data migration tools
- [ ] Backup/restore

## Future: Live Data Access

When contracts are deployed, browser could:

```typescript
// Read-only access to deployed contract data
import { network } from 'tana:network'

const balance = await network.call('contract_id', 'getBalance', ['user123'])
```

This would fetch from Tana API endpoints:
- `https://api.tana.dev/contracts/{id}/data/{key}` (read)
- `https://blockchain.tana.dev/contracts/{id}/call` (execute + write)

## Questions to Consider

1. **Data Types:** Store only strings, or support JSON/binary?
2. **Indexing:** Do we need secondary indexes or just key lookups?
3. **Privacy:** Should some data be encrypted/private?
4. **Versioning:** Should we track data history (like git)?
5. **TTL:** Do keys expire automatically?

## Next Steps

1. Implement basic localStorage version in playground
2. Add Data view tab to UI
3. Test with simple counter contract
4. Plan Redis integration for Rust runtime
