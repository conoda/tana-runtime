# Tana Storage API - Quick Start

## ✅ What's Implemented

### Playground (localStorage backend)
- ✅ Full `tana:data` API with all methods
- ✅ String and JSON object support
- ✅ Staging buffer with atomic commits
- ✅ Size validation (256B keys, 10KB values, 100KB total)
- ✅ Pattern matching with `keys('user:*')`
- ✅ TypeScript autocomplete in Monaco editor
- ✅ Browser developer tools inspection

### Rust Runtime (Redis backend)
- 🚧 Not yet implemented
- Next step: Add Redis integration

## Testing the Storage API

### Start the Playground

```bash
cd playground
npm run dev
```

Open http://localhost:4322/ in your browser.

The default code now shows a **counter contract demo** that:
1. Reads the current count from storage
2. Increments it
3. Stores metadata (timestamp, user object)
4. Commits changes atomically
5. Shows storage info

### Run It Multiple Times

Click the editor or press **Cmd+Enter** to re-run. Watch the counter increment each time!

### Inspect Storage in Browser DevTools

1. Open DevTools (F12)
2. Go to **Application > Local Storage**
3. Look for keys starting with `tana:data:`
4. See your stored data!

## API Usage

### Basic Operations

```typescript
import { data } from 'tana:data'

// Set values (staged, not committed yet)
await data.set('username', 'alice')
await data.set('user', { name: 'Bob', balance: 1000 })

// Read values (includes staged changes)
const name = await data.get('username')  // Returns: 'alice'
const user = await data.get('user')      // Returns: { name: 'Bob', ... }

// Check existence
const exists = await data.has('username')  // Returns: true

// Delete (staged)
await data.delete('username')

// Commit all changes atomically
await data.commit()
```

### Pattern Matching

```typescript
// Store user data
await data.set('user:1:name', 'Alice')
await data.set('user:2:name', 'Bob')
await data.set('user:1:balance', '500')
await data.commit()

// Find all user keys
const userKeys = await data.keys('user:*')
// Returns: ['user:1:name', 'user:2:name', 'user:1:balance']

// Find specific pattern
const balances = await data.keys('*:balance')
// Returns: ['user:1:balance']
```

### Get All Data

```typescript
const all = await data.entries()
// Returns: { 'user:1:name': 'Alice', 'user:2:name': 'Bob', ... }

console.log('Total keys:', Object.keys(all).length)
```

### Storage Limits

```typescript
console.log('Max key size:', data.MAX_KEY_SIZE)       // 256 bytes
console.log('Max value size:', data.MAX_VALUE_SIZE)   // 10,240 bytes (10 KB)
console.log('Max total size:', data.MAX_TOTAL_SIZE)   // 102,400 bytes (100 KB)
console.log('Max keys:', data.MAX_KEYS)               // 1000
```

## Example: Simple Token Contract

```typescript
import { console } from 'tana:core'
import { data } from 'tana:data'

// Initialize balances
await data.set('balance:alice', '1000')
await data.set('balance:bob', '500')

// Transfer function
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

// Execute transfer
await transfer('alice', 'bob', 200)

// Check new balances
console.log('Alice:', await data.get('balance:alice'))  // '800'
console.log('Bob:', await data.get('balance:bob'))      // '700'
```

## How It Works

### Staging Buffer

All `set()` and `delete()` operations are **staged** until you call `commit()`:

```typescript
await data.set('foo', 'bar')     // Staged
await data.set('baz', 'qux')     // Staged
// Changes not saved yet!

await data.commit()              // ✓ Now saved to localStorage
```

This ensures **atomic commits**: either all changes succeed, or none do.

### Size Validation

Commit will fail if you exceed limits:

```typescript
// This will throw an error
const bigValue = 'x'.repeat(20000)  // 20 KB
await data.set('key', bigValue)     // Throws: Value too large
```

### JSON Auto-Serialization

Objects are automatically serialized/deserialized:

```typescript
// Store object
await data.set('user', { name: 'Alice', balance: 1000 })
await data.commit()

// Retrieve as object (not string!)
const user = await data.get('user')
console.log(user.name)     // 'Alice'
console.log(user.balance)  // 1000
```

## Test Files

- `test-storage.ts` - Comprehensive API tests
- `examples/counter-contract.ts` - Simple counter example
- `playground/src/defaultCode.ts` - Default landing page demo

## Next Steps

1. **Test in Playground**: Run `cd playground && npm run dev`
2. **Inspect Storage**: Use browser DevTools
3. **Build Contracts**: Create your own smart contracts
4. **Rust Runtime**: Next milestone is Redis backend

## Future: Data View Tab

Coming soon - a UI tab in the playground to visualize storage:

```
┌─────────────────┬─────────────────┐
│                 │ [Output] [Data] │
│     Editor      ├─────────────────┤
│                 │ Key     Value   │
│                 │ counter 42      │
│                 │ user    {...}   │
└─────────────────┴─────────────────┘
```

For now, use browser DevTools to inspect localStorage!
