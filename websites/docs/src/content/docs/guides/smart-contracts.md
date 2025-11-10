---
title: Smart Contract Examples
description: Learn to write and test Tana smart contracts
sidebar:
  order: 3
---

Example contracts demonstrating the Tana blockchain runtime.

## Running Contracts from Terminal

### Quick Test
```bash
# Run the default playground example
bun run chaintest
```

### Run Any Contract
```bash
# Run a specific contract file
bun run contract examples/simple-transfer.ts

# Or use the script directly
bun scripts/run-contract.ts examples/batch-query.ts
```

## Test Files (Sanity Checks)

### `test-suite.ts` ⭐ **Recommended**
Comprehensive test suite with 20+ tests covering all major operations.

**Run:**
```bash
bun scripts/run-contract.ts examples/test-suite.ts
```

**What it tests:**
- User lookup (single & batch)
- Balance queries (single & batch)
- Query limit enforcement (max 10)
- Transfer execution & validation
- Invalid operations (self-transfer, negative/zero amounts)
- Block context availability

**Use this for quick sanity checks after making changes!**

### `alice-to-bob.ts`
Simple transfer test between two users with balance verification.

**Run:**
```bash
bun scripts/run-contract.ts examples/alice-to-bob.ts
```

**What it tests:**
- User lookup by username
- Balance queries before/after
- Simple transfer execution
- Expected balance calculations

## Example Contracts

### `default.ts`
Complete demonstration of all Tana features:
- Block context access
- Blockchain state queries
- Transaction execution
- Data storage

**Run:**
```bash
bun run chaintest
```

### `simple-transfer.ts`
Basic transfer contract showing:
- Balance queries
- Conditional logic
- Transfer execution

**Run:**
```bash
bun run contract examples/simple-transfer.ts
```

### `batch-query.ts`
Demonstrates query limitations:
- Batch queries (max 10 items)
- Security limit enforcement
- Error handling

**Run:**
```bash
bun run contract examples/batch-query.ts
```

## Writing Your Own Contracts

### Basic Structure

```typescript
import { console } from 'tana:core'
import { block } from 'tana:block'
import { tx } from 'tana:tx'

// Query blockchain state
const balance = await block.getBalance(block.executor, 'USD')
console.log(`Balance: ${balance} USD`)

// Propose state changes
if (balance >= 10) {
  tx.transfer(block.executor, 'treasury', 10, 'USD')
  const result = await tx.execute()

  if (result.success) {
    console.log('✓ Transfer complete!')
  }
}
```

### Available Modules

#### `tana:core`
```typescript
import { console, version } from 'tana:core'

console.log('Hello from contract')
console.error('Error message')
console.log(version) // { tana, deno_core, v8 }
```

#### `tana:block`
```typescript
import { block } from 'tana:block'

// Block metadata
block.height        // Current block number
block.timestamp     // Unix timestamp (ms)
block.hash          // Block hash
block.executor      // User executing contract
block.gasLimit      // Max gas
block.gasUsed       // Current gas used

// Query state (single or max 10)
await block.getBalance('alice', 'USD')
await block.getBalance(['alice', 'bob'], 'USD')
await block.getUser('alice')
await block.getUser(['alice', 'bob'])
await block.getTransaction('tx_123')
```

#### `tana:tx`
```typescript
import { tx } from 'tana:tx'

// Propose changes
tx.transfer('alice', 'bob', 10, 'USD')
tx.setBalance('treasury', 1000, 'USD')

// View pending
tx.getChanges()

// Execute
const result = await tx.execute()
```

#### `tana:data`
```typescript
import { data } from 'tana:data'

// Store data
await data.set('key', { value: 123 })
await data.commit()

// Read data
const value = await data.get('key')
```

#### `tana:utils`
```typescript
import { fetch } from 'tana:utils'

// Whitelisted fetch only
const response = await fetch('https://pokeapi.co/api/v2/pokemon/ditto')
const pokemon = await response.json()
```

## Security Limitations

### Query Limits
- **Max 10 items** per batch query
- No `getAllUsers()` or `getAllBalances()`
- Prevents blockchain enumeration attacks

### Fetch Whitelist
Only these domains are accessible:
- `pokeapi.co` (testing)
- `*.tana.dev`
- `localhost` / `127.0.0.1`

### Gas Limits
- Each contract has a gas limit (default: 1,000,000)
- Operations consume gas
- Out of gas = rollback

## Tips

### TypeScript Restrictions
The CLI executor has some TypeScript limitations:

❌ **Don't use:**
- Type annotations in catch: `catch (error: any)`
- Non-null assertions: `user!.id`
- Advanced TypeScript features

✅ **Do use:**
- Standard ES2020+ JavaScript
- Async/await
- Import statements
- Template literals

### Best Practices

**1. Query only what you need**
```typescript
// ✓ Good - specific query
const alice = await block.getUser('alice')

// ✗ Bad - can't enumerate all
const all = await block.getAllUsers() // Doesn't exist!
```

**2. Check balances before transfers**
```typescript
const balance = await block.getBalance(block.executor, 'USD')
if (balance >= amount) {
  tx.transfer(block.executor, recipient, amount, 'USD')
}
```

**3. Handle errors gracefully**
```typescript
try {
  const result = await tx.execute()
  if (!result.success) {
    console.error(`Failed: ${result.error}`)
  }
} catch (error) {
  console.error('Execution error:', error.message)
}
```

## Output

Contracts log to terminal with colored output:
- 🔵 **Blue** - Contract metadata
- 🔵 **Cyan** - Log messages
- 🔴 **Red** - Error messages
- 🟢 **Green** - Success indicators
- ⚪ **Gray** - Dividers and timestamps

Transaction results show:
- Gas used
- State changes
- Success/failure status

## Next Steps

After testing contracts locally:
1. Deploy to local blockchain node
2. Test with real ledger service
3. Deploy to testnet
4. Production deployment

See the [Quick Start Guide](/guides/quickstart/) for getting started with Tana.
