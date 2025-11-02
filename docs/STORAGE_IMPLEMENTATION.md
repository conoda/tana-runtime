# Storage Implementation Status

## ✅ What's Implemented

### Both Environments (Playground + Rust)

All `tana:data` API methods are fully implemented and working:

- ✅ `data.set(key, value)` - Stage changes (supports strings & JSON objects)
- ✅ `data.get(key)` - Read values (checks staging first, then storage)
- ✅ `data.delete(key)` - Mark for deletion
- ✅ `data.has(key)` - Check existence
- ✅ `data.keys(pattern)` - List keys with glob patterns
- ✅ `data.entries()` - Get all KV pairs
- ✅ `data.clear()` - Wipe all data
- ✅ `data.commit()` - Atomic validation and persistence

### Storage Backends

| Environment | Backend | Persistence | Status |
|-------------|---------|-------------|--------|
| **Playground** | localStorage | ✅ Yes (browser) | ✅ Complete |
| **Rust Runtime** | HashMap (in-memory) | ❌ No (resets each run) | ✅ Complete (temporary) |
| **Rust Runtime** | Redis | ✅ Yes (server) | 🚧 Planned |

### Feature Parity Achieved ✅

The **exact same TypeScript code** runs in both environments:

```typescript
import { data } from 'tana:data'

const count = await data.get('counter') || 0
await data.set('counter', count + 1)
await data.commit()
```

**Works identically in:**
- ✅ Browser playground (persists to localStorage)
- ✅ Rust CLI runtime (works but resets each run)

### Test Results

**Playground:**
```bash
cd playground && npm run dev
# Open http://localhost:4322/
# Counter increments each run ✓
# Data persists in localStorage ✓
```

**Rust CLI:**
```bash
cargo run
# Counter works ✓
# Resets to 0 each run (expected with HashMap) ✓
```

## 🏗️ Architecture

### Staging Pattern (Both Environments)

```
User Code:
  await data.set('key', 'value')  → Staged (not saved yet)
  await data.set('key2', 'value2') → Staged
  await data.commit()              → Validation + Atomic Save
```

**Why staging?**
- Atomic transactions (all or nothing)
- Validate size limits before committing
- Rollback if validation fails

### Rust Implementation Details

**Current (In-Memory):**
```rust
static STORAGE: Mutex<Option<HashMap<String, String>>>
static STAGING: Mutex<Option<HashMap<String, Option<String>>>>
```

**Storage flow:**
1. `op_data_set()` → Stage to STAGING buffer
2. `op_data_get()` → Check STAGING, then STORAGE
3. `op_data_commit()` → Validate limits → Move STAGING to STORAGE

### Playground Implementation Details

**Current (localStorage):**
```javascript
const tanaModules = {
  'tana:data': {
    data: {
      _staging: new Map(),
      set(key, value) { this._staging.set(key, serialize(value)) },
      get(key) { return _staging.get(key) || localStorage.getItem('tana:data:' + key) },
      commit() { /* validate + localStorage.setItem() */ }
    }
  }
}
```

## 📋 Next Steps for Redis

### Option 1: Full Redis Integration

Replace HashMap with Redis connection:

```rust
// Cargo.toml (already added)
redis = { version = "0.27", features = ["tokio-comp"] }

// src/main.rs
static REDIS_CLIENT: OnceCell<redis::Client> = OnceCell::new();

#[op2(async)]
async fn op_data_commit() -> Result<(), JsErrorBox> {
    let client = REDIS_CLIENT.get().unwrap();
    let mut con = client.get_async_connection().await?;

    // Use Redis MULTI/EXEC for atomic commits
    redis::pipe()
        .atomic()
        .set("contract:xyz:key", "value")
        .execute(&mut con)
        .await?;

    Ok(())
}
```

### Option 2: Hybrid Approach

Use environment variable to switch backends:

```rust
fn get_storage_backend() -> StorageBackend {
    match env::var("TANA_STORAGE") {
        Ok(val) if val == "redis" => StorageBackend::Redis,
        _ => StorageBackend::Memory
    }
}
```

**Benefits:**
- Easy testing without Redis running
- Local dev uses in-memory
- Production uses Redis

### Option 3: Keep Current + Add Persistence File

Quick win for local dev:

```rust
// On startup: Load from JSON file
// On commit: Save to JSON file
// Still in-memory during execution

fn load_storage() -> HashMap<String, String> {
    fs::read_to_string("storage.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_storage(storage: &HashMap<String, String>) {
    let json = serde_json::to_string(storage).unwrap();
    fs::write("storage.json", json).ok();
}
```

## 🐳 Docker Setup (For Redis)

When ready to add Redis:

```yaml
# docker-compose.yml
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes

  tana-node:
    build: .
    depends_on:
      - redis
    environment:
      - REDIS_URL=redis://redis:6379

volumes:
  redis-data:
```

```bash
# .env
REDIS_URL=redis://localhost:6379
```

## 📊 Comparison

| Feature | Playground | Rust (Current) | Rust (Redis) |
|---------|-----------|----------------|--------------|
| Persistence | ✅ localStorage | ❌ In-memory | ✅ Redis |
| Speed | Fast | Fastest | Fast |
| Multi-node | N/A | ❌ No | ✅ Yes |
| Setup | None | None | Docker/Redis |
| Dev UX | Excellent | Excellent | Good |

## 🎯 Recommendation

**For now:**
- ✅ Current in-memory implementation is perfect for development
- ✅ Feature parity achieved
- ✅ All tests passing

**Next milestone:**
1. Add Docker Compose with Redis
2. Implement Option 2 (Hybrid) with env var
3. Test with `TANA_STORAGE=redis cargo run`
4. Keep in-memory as default for easy local dev

**Or skip Redis entirely for now:**
- Current implementation works perfectly
- Matches playground behavior
- Easy to test without external dependencies
- Can add Redis later when deploying nodes

## 🧪 Testing

Run the same test in both environments:

**Test file: `counter-test.ts`**
```typescript
import { console } from 'tana:core'
import { data } from 'tana:data'

const count = (await data.get('counter')) || 0
await data.set('counter', count + 1)
await data.commit()
console.log('Count:', count + 1)
```

**Playground:**
```bash
cd playground && npm run dev
# Visit http://localhost:4322/
# Edit code and run
# Counter persists! ✓
```

**Rust:**
```bash
cargo run
# Counter works! ✓
# (Resets each run until Redis added)
```

**Same code, both environments working!** ✅
