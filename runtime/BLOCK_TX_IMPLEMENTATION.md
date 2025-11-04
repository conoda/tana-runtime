# Adding tana:block and tana:tx to Rust Runtime

This document outlines what needs to be added to `/runtime/src/main.rs` to achieve feature parity with the playground.

## Current Status

✅ Implemented:
- `tana:core` - console, version
- `tana:data` - storage with staging/commit
- `tana:utils` - whitelisted fetch

❌ Not Implemented:
- `tana:block` - block context and state queries
- `tana:tx` - transaction staging and execution

---

## Changes Needed

### 1. Add Global State (after line 25)

```rust
// Transaction staging (similar to data staging)
static TX_CHANGES: Mutex<Option<Vec<serde_json::Value>>> = Mutex::new(None);

// Mock block context (in production, this comes from blockchain DB)
const MOCK_BLOCK_HEIGHT: u64 = 12345;
const MOCK_BLOCK_HASH: &str = "0x1234...";  // Generate random in init
const MOCK_EXECUTOR: &str = "user_rust_runtime";
const MOCK_GAS_LIMIT: u64 = 1_000_000;

// Query limits
const MAX_BATCH_QUERY: usize = 10;
```

### 2. Add Block Context Ops (after line 296, before `main()`)

```rust
// ========== Block Context Ops ==========

#[op2(fast)]
fn op_block_get_height() -> u64 {
    MOCK_BLOCK_HEIGHT
}

#[op2(fast)]
fn op_block_get_timestamp() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64
}

#[op2]
#[string]
fn op_block_get_hash() -> String {
    MOCK_BLOCK_HASH.to_string()
}

#[op2]
#[string]
fn op_block_get_executor() -> String {
    MOCK_EXECUTOR.to_string()
}

#[op2(fast)]
fn op_block_get_gas_limit() -> u64 {
    MOCK_GAS_LIMIT
}

// ========== Blockchain State Query Ops ==========

#[op2(async)]
#[serde]
async fn op_block_get_balance(
    #[serde] user_ids: serde_json::Value,
    #[string] currency_code: String
) -> Result<serde_json::Value, deno_error::JsErrorBox> {
    // Parse input (string or array)
    let ids: Vec<String> = match user_ids {
        serde_json::Value::String(s) => vec![s],
        serde_json::Value::Array(arr) => {
            arr.into_iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        },
        _ => return Err(deno_error::JsErrorBox::new("TypeError", "Invalid user_ids")),
    };

    // Check batch limit
    if ids.len() > MAX_BATCH_QUERY {
        return Err(deno_error::JsErrorBox::new(
            "Error",
            format!("Cannot query more than {} balances at once", MAX_BATCH_QUERY)
        ));
    }

    // Fetch from ledger API
    let url = "http://localhost:8080/balances";
    let response = reqwest::get(url).await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("Failed to fetch balances: {}", e)))?;

    let balances: Vec<serde_json::Value> = response.json().await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("Failed to parse balances: {}", e)))?;

    // Find balances for each user
    let results: Vec<f64> = ids.iter().map(|user_id| {
        balances.iter()
            .find(|b| {
                b.get("ownerId").and_then(|v| v.as_str()) == Some(user_id) &&
                b.get("currencyCode").and_then(|v| v.as_str()) == Some(&currency_code)
            })
            .and_then(|b| b.get("amount"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0)
    }).collect();

    // Return single value or array based on input
    if ids.len() == 1 {
        Ok(serde_json::Value::Number(serde_json::Number::from_f64(results[0]).unwrap()))
    } else {
        Ok(serde_json::Value::Array(
            results.into_iter().map(|n| serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap())).collect()
        ))
    }
}

#[op2(async)]
#[serde]
async fn op_block_get_user(
    #[serde] user_ids: serde_json::Value
) -> Result<serde_json::Value, deno_error::JsErrorBox> {
    // Similar pattern to op_block_get_balance
    // Fetch from http://localhost:8080/users
    // Match by id or username
    // Return single or array
    todo!("Implement user query")
}

#[op2(async)]
#[serde]
async fn op_block_get_transaction(
    #[serde] tx_ids: serde_json::Value
) -> Result<serde_json::Value, deno_error::JsErrorBox> {
    // Similar pattern
    // Fetch from http://localhost:8080/transactions
    todo!("Implement transaction query")
}

// ========== Transaction Staging Ops ==========

#[op2(fast)]
fn op_tx_transfer(
    #[string] from: String,
    #[string] to: String,
    amount: f64,
    #[string] currency: String
) -> Result<(), deno_error::JsErrorBox> {
    if from == to {
        return Err(deno_error::JsErrorBox::new("Error", "Cannot transfer to self"));
    }
    if amount <= 0.0 {
        return Err(deno_error::JsErrorBox::new("Error", "Amount must be positive"));
    }

    let mut changes = TX_CHANGES.lock().unwrap();
    if changes.is_none() {
        *changes = Some(Vec::new());
    }

    let change = serde_json::json!({
        "type": "transfer",
        "from": from,
        "to": to,
        "amount": amount,
        "currency": currency
    });

    changes.as_mut().unwrap().push(change);
    Ok(())
}

#[op2(fast)]
fn op_tx_set_balance(
    #[string] user_id: String,
    amount: f64,
    #[string] currency: String
) -> Result<(), deno_error::JsErrorBox> {
    if amount < 0.0 {
        return Err(deno_error::JsErrorBox::new("Error", "Balance cannot be negative"));
    }

    let mut changes = TX_CHANGES.lock().unwrap();
    if changes.is_none() {
        *changes = Some(Vec::new());
    }

    let change = serde_json::json!({
        "type": "balance_update",
        "userId": user_id,
        "amount": amount,
        "currency": currency
    });

    changes.as_mut().unwrap().push(change);
    Ok(())
}

#[op2]
#[serde]
fn op_tx_get_changes() -> serde_json::Value {
    let changes = TX_CHANGES.lock().unwrap();
    if let Some(ref changes) = *changes {
        serde_json::Value::Array(changes.clone())
    } else {
        serde_json::Value::Array(Vec::new())
    }
}

#[op2]
#[serde]
fn op_tx_execute() -> Result<serde_json::Value, deno_error::JsErrorBox> {
    let mut changes_guard = TX_CHANGES.lock().unwrap();
    if changes_guard.is_none() {
        *changes_guard = Some(Vec::new());
    }

    let changes = changes_guard.as_ref().unwrap().clone();
    let gas_used = 100 * changes.len() as u64;

    // Check gas limit
    if gas_used > MOCK_GAS_LIMIT {
        // Rollback
        if let Some(ref mut c) = *changes_guard {
            c.clear();
        }
        return Ok(serde_json::json!({
            "success": false,
            "changes": [],
            "gasUsed": MOCK_GAS_LIMIT,
            "error": "Out of gas"
        }));
    }

    // In playground: just return success
    // In production: validate and persist to DB

    // Clear staging
    if let Some(ref mut c) = *changes_guard {
        c.clear();
    }

    Ok(serde_json::json!({
        "success": true,
        "changes": changes,
        "gasUsed": gas_used,
        "error": null
    }))
}
```

### 3. Register Ops in Extension (around line 315)

```rust
const OP_BLOCK_GET_HEIGHT: deno_core::OpDecl = op_block_get_height();
const OP_BLOCK_GET_TIMESTAMP: deno_core::OpDecl = op_block_get_timestamp();
const OP_BLOCK_GET_HASH: deno_core::OpDecl = op_block_get_hash();
const OP_BLOCK_GET_EXECUTOR: deno_core::OpDecl = op_block_get_executor();
const OP_BLOCK_GET_GAS_LIMIT: deno_core::OpDecl = op_block_get_gas_limit();
const OP_BLOCK_GET_BALANCE: deno_core::OpDecl = op_block_get_balance();
const OP_BLOCK_GET_USER: deno_core::OpDecl = op_block_get_user();
const OP_BLOCK_GET_TRANSACTION: deno_core::OpDecl = op_block_get_transaction();
const OP_TX_TRANSFER: deno_core::OpDecl = op_tx_transfer();
const OP_TX_SET_BALANCE: deno_core::OpDecl = op_tx_set_balance();
const OP_TX_GET_CHANGES: deno_core::OpDecl = op_tx_get_changes();
const OP_TX_EXECUTE: deno_core::OpDecl = op_tx_execute();

// Add to ops array in Extension
ops: std::borrow::Cow::Borrowed(&[
    // ... existing ops ...
    OP_BLOCK_GET_HEIGHT,
    OP_BLOCK_GET_TIMESTAMP,
    OP_BLOCK_GET_HASH,
    OP_BLOCK_GET_EXECUTOR,
    OP_BLOCK_GET_GAS_LIMIT,
    OP_BLOCK_GET_BALANCE,
    OP_BLOCK_GET_USER,
    OP_BLOCK_GET_TRANSACTION,
    OP_TX_TRANSFER,
    OP_TX_SET_BALANCE,
    OP_TX_GET_CHANGES,
    OP_TX_EXECUTE,
]),
```

### 4. Add JavaScript Bootstrap (around line 400)

Add to the `tanaModules` object in the bootstrap code:

```javascript
'tana:block': {
    block: {
        get height() { return globalThis.__tanaCore.ops.op_block_get_height(); },
        get timestamp() { return globalThis.__tanaCore.ops.op_block_get_timestamp(); },
        get hash() { return globalThis.__tanaCore.ops.op_block_get_hash(); },
        get previousHash() { return '0x...'; },  // Mock for now
        get executor() { return globalThis.__tanaCore.ops.op_block_get_executor(); },
        get contractId() { return 'contract_runtime'; },
        get gasLimit() { return globalThis.__tanaCore.ops.op_block_get_gas_limit(); },
        get gasUsed() { return 0; },  // Track this
        MAX_BATCH_QUERY: 10,

        async getBalance(userIds, currencyCode) {
            return await globalThis.__tanaCore.ops.op_block_get_balance(userIds, currencyCode);
        },

        async getUser(userIds) {
            return await globalThis.__tanaCore.ops.op_block_get_user(userIds);
        },

        async getTransaction(txIds) {
            return await globalThis.__tanaCore.ops.op_block_get_transaction(txIds);
        }
    }
},

'tana:tx': {
    tx: {
        transfer(from, to, amount, currency) {
            globalThis.__tanaCore.ops.op_tx_transfer(from, to, amount, currency);
        },

        setBalance(userId, amount, currency) {
            globalThis.__tanaCore.ops.op_tx_set_balance(userId, amount, currency);
        },

        getChanges() {
            return globalThis.__tanaCore.ops.op_tx_get_changes();
        },

        async execute() {
            return globalThis.__tanaCore.ops.op_tx_execute();
        }
    }
}
```

---

## Testing

After implementation:

```bash
# Test with full default example
bun run chaintest:full

# Should execute successfully with block context and transactions
```

---

## Estimated Effort

- **Time**: 2-3 hours
- **Complexity**: Medium (following existing patterns)
- **Testing**: Straightforward (same tests as playground)

---

## Alternative: Use Shared Implementation

Instead of duplicating code, consider:
1. Create a shared `tana-modules` crate
2. Define the module interfaces
3. Implement once, use in both playground and runtime

This ensures true parity and reduces maintenance.
