# Contract Execution Model

> Design for block-based contract execution with transaction semantics
> Created: 2025-11-02

---

## Overview

Contracts execute within a **block context** and propose **state changes** that are validated and committed via `execute()`.

### Key Principles

1. **Block Context** - Every execution has access to current block state
2. **Transaction Semantics** - State changes are staged, validated, then committed
3. **Parity** - Same behavior in playground and runtime
4. **Deterministic** - Same inputs = same outputs

---

## Block Context API

### `tana:block` Module

```typescript
declare module 'tana:block' {
  export const block: {
    // Current block information
    readonly height: number;        // Current block number
    readonly timestamp: number;     // Unix timestamp (ms)
    readonly hash: string;          // Current block hash
    readonly previousHash: string;  // Previous block hash

    // Execution context
    readonly executor: string;      // User ID executing this contract
    readonly contractId: string;    // This contract's ID
    readonly gasLimit: number;      // Max execution units
    readonly gasUsed: number;       // Current gas consumed
    readonly MAX_BATCH_QUERY: 10;   // Max items per batch query

    // State query methods (single or batch up to MAX_BATCH_QUERY)
    getBalance(userIds: string | string[], currencyCode: string): Promise<number | number[]>;
    getUser(userIds: string | string[]): Promise<User | null | (User | null)[]>;
    getTransaction(txIds: string | string[]): Promise<Transaction | null | (Transaction | null)[]>;
  };
}
```

### Example Usage

```typescript
import { block } from 'tana:block';
import { console } from 'tana:core';

console.log(`Executing at block ${block.height}`);
console.log(`Executor: ${block.executor}`);
console.log(`Timestamp: ${new Date(block.timestamp).toISOString()}`);

// Query single user
const alice = await block.getUser('alice');
if (alice) {
  const balance = await block.getBalance(alice.id, 'USD');
  console.log(`Alice's balance: ${balance} USD`);
}

// Batch query (max 10 items)
const users = await block.getUser(['alice', 'bob', 'charlie']);
const balances = await block.getBalance(['alice', 'bob'], 'USD');
```

### Query Limitations

**Anti-Abuse Protection:**
- All query methods accept **single ID** or **array of max 10 IDs**
- Prevents full blockchain enumeration attacks
- Throws error if querying more than `MAX_BATCH_QUERY` items
- No `getAllUsers()` or `getAllBalances()` methods available

**Rationale:**
- Prevents denial-of-service by limiting query scope
- Encourages efficient, targeted queries
- Contracts should know which users/transactions they need
- Blockchain state is not meant to be fully enumerable by contracts

---

## Transaction Execution API

### `tana:tx` Module

```typescript
declare module 'tana:tx' {
  export const tx: {
    // State change operations
    transfer(from: string, to: string, amount: number, currency: string): void;
    setBalance(userId: string, amount: number, currency: string): void;

    // Get proposed changes (read-only)
    getChanges(): TransactionChange[];

    // Validate and commit all changes
    execute(): Promise<TransactionResult>;
  };

  interface TransactionChange {
    type: 'transfer' | 'balance_update' | 'data_update';
    from?: string;
    to?: string;
    amount?: number;
    currency?: string;
    key?: string;
    value?: any;
  }

  interface TransactionResult {
    success: boolean;
    changes: TransactionChange[];
    gasUsed: number;
    error?: string;
  }
}
```

### Example Usage

```typescript
import { tx } from 'tana:tx';
import { data } from 'tana:data';
import { console } from 'tana:core';

// Propose state changes
tx.transfer('alice', 'bob', 10, 'USD');
await data.set('lastTransfer', { from: 'alice', to: 'bob', amount: 10 });

// Commit changes atomically
const result = await tx.execute();

if (result.success) {
  console.log('Transaction executed successfully!');
  console.log(`Gas used: ${result.gasUsed}`);
  console.log(`Changes: ${JSON.stringify(result.changes)}`);
} else {
  console.error(`Transaction failed: ${result.error}`);
}
```

---

## Execution Flow

### Playground Execution

```
1. User writes contract code
2. Code runs in sandbox with mock block context
3. State changes are staged in memory
4. execute() validates changes and displays them
5. No actual persistence (just UI display)
```

### Runtime Execution

```
1. Contract deployed to blockchain
2. User calls contract via CLI/API
3. Runtime creates execution context with real block data
4. Code runs in V8 sandbox
5. execute() validates changes
6. Changes are written to PostgreSQL
7. New block is created with state root
```

---

## Implementation Plan

### Phase 1: Block Context

**Playground:**
- Add mock `tana:block` module to sandbox
- Mock block height, timestamp, hash
- Mock executor ID (test user)

**Runtime:**
- Implement `tana:block` in Rust runtime
- Query current block from database
- Inject context into V8 isolate

**Files to modify:**
- `website/src/pages/sandbox.astro` - Add tana:block module
- `website/src/components/Editor.svelte` - Add TypeScript definitions
- `runtime/src/lib.rs` - Implement block context (future)

### Phase 2: Transaction API

**Playground:**
- Add `tana:tx` module to sandbox
- Stage changes in memory
- `execute()` returns mock result
- Display changes in UI (new tab?)

**Runtime:**
- Implement `tana:tx` in Rust runtime
- Validate balance constraints
- Write to PostgreSQL transaction table
- Update account balances atomically

**Files to modify:**
- `website/src/pages/sandbox.astro` - Add tana:tx module
- `website/src/components/StateViewer.svelte` - Add "Pending Changes" tab
- `ledger/src/transactions.rs` - Transaction validation (future)

---

## State Change Validation

### Validation Rules

1. **Balance Transfers:**
   - Source has sufficient balance
   - Amounts are positive
   - Currency exists
   - No self-transfers

2. **Data Updates:**
   - Key size limits (256 bytes)
   - Value size limits (10 KB)
   - Total storage limits (100 KB)
   - Valid key patterns

3. **Gas Limits:**
   - Execution within gas limit
   - Complex operations cost more gas
   - Out of gas = rollback all changes

### Error Handling

```typescript
try {
  tx.transfer('alice', 'bob', 1000000, 'USD');
  const result = await tx.execute();
} catch (error) {
  // Insufficient balance, invalid currency, etc.
  console.error(error.message);
}
```

---

## UI Changes for Playground

### New Tab: "Pending Changes"

Show staged changes before `execute()` is called:

```
Pending Changes (3)
├─ Transfer: alice → bob (10 USD)
├─ Data Update: lastTransfer = {...}
└─ Balance Update: alice = 90 USD
```

After `execute()`:

```
✓ Transaction Successful
Gas Used: 1,234 units
Changes Applied:
├─ ✓ Transfer: alice → bob (10 USD)
├─ ✓ Data Update: lastTransfer = {...}
└─ ✓ Balance Update: alice = 90 USD
```

---

## Example Contract

### Simple Transfer Contract

```typescript
import { block } from 'tana:block';
import { tx } from 'tana:tx';
import { data } from 'tana:data';
import { console } from 'tana:core';

// Log execution context
console.log(`Block: ${block.height}`);
console.log(`Executor: ${block.executor}`);

// Query current balance (blockchain state)
const currentBalance = await block.getBalance(block.executor, 'USD');
console.log(`Current balance: ${currentBalance} USD`);

// Only transfer if sufficient funds
if (currentBalance >= 5) {
  // Read previous state
  const history = await data.get('transfers') || [];

  // Propose state changes
  tx.transfer(block.executor, 'treasury', 5, 'USD');

  // Update history
  history.push({
    from: block.executor,
    to: 'treasury',
    amount: 5,
    timestamp: block.timestamp,
    blockHeight: block.height
  });

  await data.set('transfers', history);
  await data.commit();

  // Execute transaction
  const result = await tx.execute();

  if (result.success) {
    console.log('✓ Transfer complete!');
  } else {
    console.error(`✗ Transfer failed: ${result.error}`);
  }
} else {
  console.error('✗ Insufficient balance');
}
```

---

## Parity Checklist

- [ ] Same `tana:block` API in playground and runtime
- [ ] Same `tana:tx` API in playground and runtime
- [ ] Same validation rules in both environments
- [ ] Same error messages
- [ ] Same gas metering (future)
- [ ] Playground can test code that will run in production

---

## Next Steps

1. Implement `tana:block` in sandbox (playground)
2. Add TypeScript definitions to Monaco
3. Implement `tana:tx` in sandbox (playground)
4. Add "Pending Changes" tab to StateViewer
5. Test with example contracts
6. Document for runtime implementation

---

**Status:** Design approved, ready for implementation
**Last Updated:** 2025-11-02
