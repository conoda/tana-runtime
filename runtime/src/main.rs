use std::fs;
use std::sync::Mutex;
use std::collections::HashMap;

use deno_core::op2;
use deno_core::{
    Extension,
    JsRuntime,
    ModuleCodeString,
    RuntimeOptions,
};

// Global storage (in-memory HashMap, matches playground localStorage)
// In production, this will be replaced with Redis
static STORAGE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

// Global staging buffer for uncommitted changes
// Maps keys to Option<String>: Some(value) = set, None = delete
static STAGING: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

// Storage limits (same as playground)
const MAX_KEY_SIZE: usize = 256;
const MAX_VALUE_SIZE: usize = 10_240;  // 10 KB
const MAX_TOTAL_SIZE: usize = 102_400; // 100 KB
const MAX_KEYS: usize = 1000;

// Transaction staging (for tana:tx module)
static TX_CHANGES: Mutex<Option<Vec<serde_json::Value>>> = Mutex::new(None);

// Mock block context (in production, this comes from blockchain DB)
const MOCK_BLOCK_HEIGHT: u64 = 12345;
const MOCK_EXECUTOR: &str = "user_rust_runtime";
const MOCK_CONTRACT_ID: &str = "contract_rust";
const MOCK_GAS_LIMIT: u64 = 1_000_000;
static MOCK_GAS_USED: Mutex<u64> = Mutex::new(0);

// Query limits (anti-abuse)
const MAX_BATCH_QUERY: usize = 10;

#[op2]
fn op_sum(#[serde] nums: Vec<f64>) -> Result<f64, deno_error::JsErrorBox> {
    Ok(nums.iter().sum())
}

#[op2(fast)]
fn op_print_stderr(#[string] msg: String) {
    eprint!("{}", msg);
}

// Whitelisted domains matching the playground
const ALLOWED_DOMAINS: &[&str] = &[
    "pokeapi.co",           // Testing until Tana infra is ready
    "tana.dev",             // Tana domains
    "api.tana.dev",
    "blockchain.tana.dev",
    "localhost",            // Local development
    "127.0.0.1",
];

#[op2(async)]
#[string]
async fn op_fetch(#[string] url: String) -> Result<String, deno_error::JsErrorBox> {
    // Parse URL
    let parsed = reqwest::Url::parse(&url)
        .map_err(|e| deno_error::JsErrorBox::new("TypeError", format!("Invalid URL: {}", e)))?;

    // Check domain whitelist
    let hostname = parsed.host_str()
        .ok_or_else(|| deno_error::JsErrorBox::new("TypeError", "Invalid hostname"))?;

    let is_allowed = ALLOWED_DOMAINS.iter().any(|domain| {
        hostname == *domain || hostname.ends_with(&format!(".{}", domain))
    });

    if !is_allowed {
        return Err(deno_error::JsErrorBox::new(
            "Error",
            format!(
                "fetch blocked: domain \"{}\" not in whitelist. Allowed domains: {}",
                hostname,
                ALLOWED_DOMAINS.join(", ")
            )
        ));
    }

    // Perform fetch
    let response = reqwest::get(&url).await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("fetch failed: {}", e)))?;

    let body = response.text().await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("failed to read response body: {}", e)))?;

    Ok(body)
}

// ========== Data Storage Ops ==========

#[op2(fast)]
#[string]
fn op_data_set(#[string] key: String, #[string] value: String) -> Result<(), deno_error::JsErrorBox> {
    // Validate key size
    if key.len() > MAX_KEY_SIZE {
        return Err(deno_error::JsErrorBox::new(
            "Error",
            format!("Key too large: {} bytes (max {})", key.len(), MAX_KEY_SIZE)
        ));
    }

    // Validate value size
    if value.len() > MAX_VALUE_SIZE {
        return Err(deno_error::JsErrorBox::new(
            "Error",
            format!("Value too large: {} bytes (max {})", value.len(), MAX_VALUE_SIZE)
        ));
    }

    // Initialize staging if needed
    let mut staging = STAGING.lock().unwrap();
    if staging.is_none() {
        *staging = Some(HashMap::new());
    }

    // Stage the change
    staging.as_mut().unwrap().insert(key, Some(value));

    Ok(())
}

#[op2]
#[string]
fn op_data_get(#[string] key: String) -> Result<Option<String>, deno_error::JsErrorBox> {
    // Check staging first
    let staging = STAGING.lock().unwrap();
    if let Some(ref stage) = *staging {
        if let Some(staged_value) = stage.get(&key) {
            return Ok(staged_value.clone());
        }
    }

    // Then check storage
    let storage = STORAGE.lock().unwrap();
    if let Some(ref store) = *storage {
        return Ok(store.get(&key).cloned());
    }

    Ok(None)
}

#[op2(fast)]
fn op_data_delete(#[string] key: String) -> Result<(), deno_error::JsErrorBox> {
    // Initialize staging if needed
    let mut staging = STAGING.lock().unwrap();
    if staging.is_none() {
        *staging = Some(HashMap::new());
    }

    // Mark for deletion
    staging.as_mut().unwrap().insert(key, None);

    Ok(())
}

#[op2(fast)]
fn op_data_has(#[string] key: String) -> Result<bool, deno_error::JsErrorBox> {
    // Check staging first
    let staging = STAGING.lock().unwrap();
    if let Some(ref stage) = *staging {
        if let Some(staged_value) = stage.get(&key) {
            return Ok(staged_value.is_some());
        }
    }

    // Then check storage
    let storage = STORAGE.lock().unwrap();
    if let Some(ref store) = *storage {
        return Ok(store.contains_key(&key));
    }

    Ok(false)
}

#[op2]
#[serde]
fn op_data_keys(#[string] pattern: Option<String>) -> Result<Vec<String>, deno_error::JsErrorBox> {
    use std::collections::HashSet;

    let mut all_keys = HashSet::new();

    // Get keys from storage
    let storage = STORAGE.lock().unwrap();
    if let Some(ref store) = *storage {
        for key in store.keys() {
            all_keys.insert(key.clone());
        }
    }

    // Merge with staging (add new keys, remove deleted ones)
    let staging = STAGING.lock().unwrap();
    if let Some(ref stage) = *staging {
        for (key, value) in stage.iter() {
            if value.is_none() {
                all_keys.remove(key);
            } else {
                all_keys.insert(key.clone());
            }
        }
    }

    let mut keys: Vec<String> = all_keys.into_iter().collect();

    // Apply pattern filter if provided
    if let Some(pattern_str) = pattern {
        let regex_pattern = pattern_str.replace("*", ".*");
        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))
            .map_err(|e| deno_error::JsErrorBox::new("Error", format!("Invalid pattern: {}", e)))?;
        keys.retain(|k| regex.is_match(k));
    }

    keys.sort();
    Ok(keys)
}

#[op2(fast)]
fn op_data_clear() -> Result<(), deno_error::JsErrorBox> {
    // Clear storage
    let mut storage = STORAGE.lock().unwrap();
    if let Some(ref mut store) = *storage {
        store.clear();
    }

    // Clear staging
    let mut staging = STAGING.lock().unwrap();
    if let Some(ref mut stage) = *staging {
        stage.clear();
    }

    Ok(())
}

#[op2(fast)]
fn op_data_commit() -> Result<(), deno_error::JsErrorBox> {
    // Initialize storage if needed
    let mut storage = STORAGE.lock().unwrap();
    if storage.is_none() {
        *storage = Some(HashMap::new());
    }

    let store = storage.as_mut().unwrap();

    // Calculate total size after commit
    let mut total_size = 0;
    let mut total_keys = 0;

    // Count existing non-deleted keys
    let staging = STAGING.lock().unwrap();
    let empty_map = HashMap::new();
    let stage = staging.as_ref().unwrap_or(&empty_map);

    for (key, value) in store.iter() {
        // Skip if marked for deletion in staging
        if stage.get(key).map_or(false, |v| v.is_none()) {
            continue;
        }
        total_size += key.len() + value.len();
        total_keys += 1;
    }

    // Add staged changes
    for (key, value) in stage.iter() {
        if let Some(ref val) = value {
            total_size += key.len() + val.len();
            if !store.contains_key(key) {
                total_keys += 1;
            }
        }
    }

    // Validate limits
    if total_size > MAX_TOTAL_SIZE {
        return Err(deno_error::JsErrorBox::new(
            "Error",
            format!("Storage limit exceeded: {} bytes (max {})", total_size, MAX_TOTAL_SIZE)
        ));
    }

    if total_keys > MAX_KEYS {
        return Err(deno_error::JsErrorBox::new(
            "Error",
            format!("Too many keys: {} (max {})", total_keys, MAX_KEYS)
        ));
    }

    // Commit all staged changes
    for (key, value) in stage.iter() {
        if let Some(ref val) = value {
            store.insert(key.clone(), val.clone());
        } else {
            store.remove(key);
        }
    }

    // Clear staging after successful commit
    drop(staging);
    let mut staging = STAGING.lock().unwrap();
    if let Some(ref mut stage) = *staging {
        stage.clear();
    }

    Ok(())
}

// ========== Block Context Ops ==========

#[op2(fast)]
#[bigint]
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
    // Generate a mock hash (in production, this comes from blockchain)
    format!("0x{:x}", MOCK_BLOCK_HEIGHT)
}

#[op2]
#[string]
fn op_block_get_previous_hash() -> String {
    // Generate a mock previous hash
    format!("0x{:x}", MOCK_BLOCK_HEIGHT - 1)
}

#[op2]
#[string]
fn op_block_get_executor() -> String {
    MOCK_EXECUTOR.to_string()
}

#[op2]
#[string]
fn op_block_get_contract_id() -> String {
    MOCK_CONTRACT_ID.to_string()
}

#[op2(fast)]
#[bigint]
fn op_block_get_gas_limit() -> u64 {
    MOCK_GAS_LIMIT
}

#[op2(fast)]
#[bigint]
fn op_block_get_gas_used() -> u64 {
    *MOCK_GAS_USED.lock().unwrap()
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
        Ok(serde_json::json!(results[0]))
    } else {
        Ok(serde_json::json!(results))
    }
}

#[op2(async)]
#[serde]
async fn op_block_get_user(
    #[serde] user_ids: serde_json::Value
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
            format!("Cannot query more than {} users at once", MAX_BATCH_QUERY)
        ));
    }

    // Fetch from ledger API
    let url = "http://localhost:8080/users";
    let response = reqwest::get(url).await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("Failed to fetch users: {}", e)))?;

    let users: Vec<serde_json::Value> = response.json().await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("Failed to parse users: {}", e)))?;

    // Find users by id or username
    let results: Vec<Option<serde_json::Value>> = ids.iter().map(|user_id| {
        users.iter()
            .find(|u| {
                u.get("id").and_then(|v| v.as_str()) == Some(user_id) ||
                u.get("username").and_then(|v| v.as_str()) == Some(user_id)
            })
            .cloned()
    }).collect();

    // Return single value or array based on input
    if ids.len() == 1 {
        Ok(results[0].clone().unwrap_or(serde_json::Value::Null))
    } else {
        Ok(serde_json::json!(results))
    }
}

#[op2(async)]
#[serde]
async fn op_block_get_transaction(
    #[serde] tx_ids: serde_json::Value
) -> Result<serde_json::Value, deno_error::JsErrorBox> {
    // Parse input (string or array)
    let ids: Vec<String> = match tx_ids {
        serde_json::Value::String(s) => vec![s],
        serde_json::Value::Array(arr) => {
            arr.into_iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        },
        _ => return Err(deno_error::JsErrorBox::new("TypeError", "Invalid tx_ids")),
    };

    // Check batch limit
    if ids.len() > MAX_BATCH_QUERY {
        return Err(deno_error::JsErrorBox::new(
            "Error",
            format!("Cannot query more than {} transactions at once", MAX_BATCH_QUERY)
        ));
    }

    // Fetch from ledger API
    let url = "http://localhost:8080/transactions";
    let response = reqwest::get(url).await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("Failed to fetch transactions: {}", e)))?;

    let transactions: Vec<serde_json::Value> = response.json().await
        .map_err(|e| deno_error::JsErrorBox::new("Error", format!("Failed to parse transactions: {}", e)))?;

    // Find transactions by id
    let results: Vec<Option<serde_json::Value>> = ids.iter().map(|tx_id| {
        transactions.iter()
            .find(|tx| tx.get("id").and_then(|v| v.as_str()) == Some(tx_id))
            .cloned()
    }).collect();

    // Return single value or array based on input
    if ids.len() == 1 {
        Ok(results[0].clone().unwrap_or(serde_json::Value::Null))
    } else {
        Ok(serde_json::json!(results))
    }
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

    // Update global gas used
    let mut global_gas = MOCK_GAS_USED.lock().unwrap();
    let new_gas_total = *global_gas + gas_used;

    // Check gas limit
    if new_gas_total > MOCK_GAS_LIMIT {
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

    // Update gas used
    *global_gas = new_gas_total;

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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let total_start = std::time::Instant::now();

    // Get contract file from command line args (defaults to example.ts)
    let args: Vec<String> = std::env::args().collect();
    let contract_file = if args.len() > 1 {
        &args[1]
    } else {
        "example.ts"
    };

    // Check for pre-compiled .js version
    let (file_path, is_precompiled) = if contract_file.ends_with(".ts") {
        let js_version = contract_file.replace(".ts", ".js");
        if std::path::Path::new(&js_version).exists() {
            eprintln!("[RUNTIME] Using pre-compiled: {}", js_version);
            (js_version, true)
        } else {
            eprintln!("[RUNTIME] Using TypeScript: {}", contract_file);
            (contract_file.to_string(), false)
        }
    } else if contract_file.ends_with(".js") {
        eprintln!("[RUNTIME] Using pre-compiled: {}", contract_file);
        (contract_file.to_string(), true)
    } else {
        // Try both .js and .ts
        let js_path = format!("{}.js", contract_file);
        let ts_path = format!("{}.ts", contract_file);
        if std::path::Path::new(&js_path).exists() {
            eprintln!("[RUNTIME] Using pre-compiled: {}", js_path);
            (js_path, true)
        } else if std::path::Path::new(&ts_path).exists() {
            eprintln!("[RUNTIME] Using TypeScript: {}", ts_path);
            (ts_path, false)
        } else {
            panic!("Contract not found: {} (tried .js and .ts)", contract_file);
        }
    };

    // 1) expose our ops
    let ext_start = std::time::Instant::now();
    const OP_SUM: deno_core::OpDecl = op_sum();
    const OP_PRINT_STDERR: deno_core::OpDecl = op_print_stderr();
    const OP_FETCH: deno_core::OpDecl = op_fetch();
    const OP_DATA_SET: deno_core::OpDecl = op_data_set();
    const OP_DATA_GET: deno_core::OpDecl = op_data_get();
    const OP_DATA_DELETE: deno_core::OpDecl = op_data_delete();
    const OP_DATA_HAS: deno_core::OpDecl = op_data_has();
    const OP_DATA_KEYS: deno_core::OpDecl = op_data_keys();
    const OP_DATA_CLEAR: deno_core::OpDecl = op_data_clear();
    const OP_DATA_COMMIT: deno_core::OpDecl = op_data_commit();

    // Block context ops
    const OP_BLOCK_GET_HEIGHT: deno_core::OpDecl = op_block_get_height();
    const OP_BLOCK_GET_TIMESTAMP: deno_core::OpDecl = op_block_get_timestamp();
    const OP_BLOCK_GET_HASH: deno_core::OpDecl = op_block_get_hash();
    const OP_BLOCK_GET_PREVIOUS_HASH: deno_core::OpDecl = op_block_get_previous_hash();
    const OP_BLOCK_GET_EXECUTOR: deno_core::OpDecl = op_block_get_executor();
    const OP_BLOCK_GET_CONTRACT_ID: deno_core::OpDecl = op_block_get_contract_id();
    const OP_BLOCK_GET_GAS_LIMIT: deno_core::OpDecl = op_block_get_gas_limit();
    const OP_BLOCK_GET_GAS_USED: deno_core::OpDecl = op_block_get_gas_used();

    // State query ops
    const OP_BLOCK_GET_BALANCE: deno_core::OpDecl = op_block_get_balance();
    const OP_BLOCK_GET_USER: deno_core::OpDecl = op_block_get_user();
    const OP_BLOCK_GET_TRANSACTION: deno_core::OpDecl = op_block_get_transaction();

    // Transaction ops
    const OP_TX_TRANSFER: deno_core::OpDecl = op_tx_transfer();
    const OP_TX_SET_BALANCE: deno_core::OpDecl = op_tx_set_balance();
    const OP_TX_GET_CHANGES: deno_core::OpDecl = op_tx_get_changes();
    const OP_TX_EXECUTE: deno_core::OpDecl = op_tx_execute();

    let ext = Extension {
        name: "tana_ext",
        ops: std::borrow::Cow::Borrowed(&[
            OP_SUM,
            OP_PRINT_STDERR,
            OP_FETCH,
            OP_DATA_SET,
            OP_DATA_GET,
            OP_DATA_DELETE,
            OP_DATA_HAS,
            OP_DATA_KEYS,
            OP_DATA_CLEAR,
            OP_DATA_COMMIT,
            OP_BLOCK_GET_HEIGHT,
            OP_BLOCK_GET_TIMESTAMP,
            OP_BLOCK_GET_HASH,
            OP_BLOCK_GET_PREVIOUS_HASH,
            OP_BLOCK_GET_EXECUTOR,
            OP_BLOCK_GET_CONTRACT_ID,
            OP_BLOCK_GET_GAS_LIMIT,
            OP_BLOCK_GET_GAS_USED,
            OP_BLOCK_GET_BALANCE,
            OP_BLOCK_GET_USER,
            OP_BLOCK_GET_TRANSACTION,
            OP_TX_TRANSFER,
            OP_TX_SET_BALANCE,
            OP_TX_GET_CHANGES,
            OP_TX_EXECUTE,
        ]),
        ..Default::default()
    };
    eprintln!("  [TIMING] Extension setup: {}ms", ext_start.elapsed().as_millis());

    // 2) runtime – NO custom module loader for now
    let runtime_start = std::time::Instant::now();
    let mut runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![ext],
        // we'll just use the default loader (= scripts only)
        module_loader: None,
        ..Default::default()
    });
    eprintln!("  [TIMING] V8 runtime creation: {}ms", runtime_start.elapsed().as_millis());

    // 3) load TS compiler (only if not pre-compiled)
    if !is_precompiled {
        let ts_load_start = std::time::Instant::now();
        let ts_src = fs::read_to_string("typescript.js")
            .expect("missing typescript.js");
        runtime
            .execute_script("typescript.js", ModuleCodeString::from(ts_src))
            .expect("load ts");
        eprintln!("  [TIMING] TypeScript compiler load: {}ms", ts_load_start.elapsed().as_millis());
    } else {
        eprintln!("  [TIMING] TypeScript compiler load: 0ms (pre-compiled JS)");
    }

    // 4) load bootstrap (conditional based on whether contract is pre-compiled)
    let bootstrap_start = std::time::Instant::now();

    let tana_version = env!("CARGO_PKG_VERSION");
    let deno_core_version = env!("DENO_CORE_VERSION");
    let v8_version = env!("V8_VERSION");

    if !is_precompiled {
        // Full bootstrap with tana-globals transpilation
        let tana_globals = fs::read_to_string("tana-globals.ts")
            .expect("missing tana-globals.ts");

        let bootstrap_globals = format!(
            r#"
            // 1. FIRST: Stash Deno.core before we delete it
            globalThis.__tanaCore = globalThis.Deno?.core;

            // 2. Delete Deno to create sandbox
            delete globalThis.Deno;

            // 3. NOW we can safely define modules that use __tanaCore
            const tanaModules = Object.create(null);

            // core module - browser-like console API
            tanaModules["tana/core"] = {{
                console: {{
                    log(...args) {{
                        if (globalThis.__tanaCore) {{
                            const msg = args.map(v => {{
                                if (typeof v === 'object') {{
                                    try {{ return JSON.stringify(v, null, 2); }}
                                    catch {{ return String(v); }}
                                }}
                                return String(v);
                            }}).join(' ');
                            globalThis.__tanaCore.print(msg + "\n");
                        }}
                    }},
                    error(...args) {{
                        if (globalThis.__tanaCore) {{
                            const msg = args.map(v => {{
                                if (typeof v === 'object') {{
                                    try {{ return JSON.stringify(v, null, 2); }}
                                    catch {{ return String(v); }}
                                }}
                                return String(v);
                            }}).join(' ');
                            globalThis.__tanaCore.ops.op_print_stderr(msg + "\n");
                        }}
                    }},
                }},
                version: {{
                    tana: "{tana_version}",
                    deno_core: "{deno_core_version}",
                    v8: "{v8_version}",
                }},
            }};

            // utils module - whitelisted fetch API
            tanaModules["tana/utils"] = {{
                async fetch(url) {{
                    if (!globalThis.__tanaCore) {{
                        throw new Error('Tana runtime not initialized');
                    }}
                    const result = await globalThis.__tanaCore.ops.op_fetch(url);
                    // Return a Response-like object
                    return {{
                        ok: true,
                        status: 200,
                        async text() {{ return result; }},
                        async json() {{ return JSON.parse(result); }}
                    }};
                }}
            }};

            // data module - persistent KV storage
            tanaModules["tana/data"] = {{
                data: {{
                    MAX_KEY_SIZE: 256,
                    MAX_VALUE_SIZE: 10240,
                    MAX_TOTAL_SIZE: 102400,
                    MAX_KEYS: 1000,

                    // Helper: serialize value (supports strings, objects, and BigInt)
                    _serialize(value) {{
                        if (typeof value === 'string') {{
                            return value;
                        }}
                        // Use replacer to convert BigInt to string
                        return JSON.stringify(value, (key, val) => {{
                            if (typeof val === 'bigint') {{
                                return val.toString();
                            }}
                            return val;
                        }});
                    }},

                    // Helper: deserialize value (returns original type)
                    _deserialize(value) {{
                        if (value === null) return null;
                        try {{
                            return JSON.parse(value);
                        }} catch {{
                            return value; // Return as string if not JSON
                        }}
                    }},

                    async set(key, value) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        const serialized = this._serialize(value);
                        globalThis.__tanaCore.ops.op_data_set(key, serialized);
                    }},

                    async get(key) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        const value = globalThis.__tanaCore.ops.op_data_get(key);
                        return this._deserialize(value);
                    }},

                    async delete(key) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        globalThis.__tanaCore.ops.op_data_delete(key);
                    }},

                    async has(key) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_data_has(key);
                    }},

                    async keys(pattern) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_data_keys(pattern || null);
                    }},

                    async entries() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        const allKeys = await this.keys();
                        const result = {{}};
                        for (const key of allKeys) {{
                            result[key] = await this.get(key);
                        }}
                        return result;
                    }},

                    async clear() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        globalThis.__tanaCore.ops.op_data_clear();
                    }},

                    async commit() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        globalThis.__tanaCore.ops.op_data_commit();
                    }}
                }}
            }};

            // block module - block context and state queries
            tanaModules["tana/block"] = {{
                block: {{
                    get height() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_height();
                    }},

                    get timestamp() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_timestamp();
                    }},

                    get hash() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_hash();
                    }},

                    get previousHash() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_previous_hash();
                    }},

                    get executor() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_executor();
                    }},

                    get contractId() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_contract_id();
                    }},

                    get gasLimit() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_gas_limit();
                    }},

                    get gasUsed() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_block_get_gas_used();
                    }},

                    MAX_BATCH_QUERY: 10,

                    async getBalance(userIds, currencyCode) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return await globalThis.__tanaCore.ops.op_block_get_balance(userIds, currencyCode);
                    }},

                    async getUser(userIds) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return await globalThis.__tanaCore.ops.op_block_get_user(userIds);
                    }},

                    async getTransaction(txIds) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return await globalThis.__tanaCore.ops.op_block_get_transaction(txIds);
                    }}
                }}
            }};

            // tx module - transaction staging and execution
            tanaModules["tana/tx"] = {{
                tx: {{
                    transfer(from, to, amount, currency) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        globalThis.__tanaCore.ops.op_tx_transfer(from, to, amount, currency);
                    }},

                    setBalance(userId, amount, currency) {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        globalThis.__tanaCore.ops.op_tx_set_balance(userId, amount, currency);
                    }},

                    getChanges() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_tx_get_changes();
                    }},

                    async execute() {{
                        if (!globalThis.__tanaCore) {{
                            throw new Error('Tana runtime not initialized');
                        }}
                        return globalThis.__tanaCore.ops.op_tx_execute();
                    }}
                }}
            }};

            // 4. Load user-defined globals (your TS)
            (function () {{
              const src = {tana_src};
              const out = ts.transpileModule(src, {{
                compilerOptions: {{
                  target: "ES2020",
                  module: ts.ModuleKind.ESNext
                }}
              }});
              (0, eval)(out.outputText);
            }})();

            // 5. Import shim
            globalThis.__tanaImport = function (spec) {{
              const m = tanaModules[spec];
              if (!m) throw new Error("unknown tana module: " + spec);
              return m;
            }};
            "#,
            tana_src = serde_json::to_string(&tana_globals).unwrap(),
            tana_version = tana_version,
            deno_core_version = deno_core_version,
            v8_version = v8_version,
        );

        runtime
            .execute_script("tana-bootstrap.js", ModuleCodeString::from(bootstrap_globals))
            .expect("bootstrap tana globals");
    } else {
        // Lightweight bootstrap for pre-compiled JS (no transpilation needed)
        let simple_bootstrap = format!(
            r#"
            // 1. Stash Deno.core before we delete it
            globalThis.__tanaCore = globalThis.Deno?.core;

            // 2. Delete Deno to create sandbox
            delete globalThis.Deno;

            // 3. Define tana modules directly (no transpilation)
            const tanaModules = Object.create(null);

            // core module
            tanaModules["tana/core"] = {{
                console: {{
                    log(...args) {{
                        if (globalThis.__tanaCore) {{
                            const msg = args.map(v => {{
                                if (typeof v === 'object') {{
                                    try {{ return JSON.stringify(v, null, 2); }}
                                    catch {{ return String(v); }}
                                }}
                                return String(v);
                            }}).join(' ');
                            globalThis.__tanaCore.print(msg + "\n");
                        }}
                    }},
                    error(...args) {{
                        if (globalThis.__tanaCore) {{
                            const msg = args.map(v => {{
                                if (typeof v === 'object') {{
                                    try {{ return JSON.stringify(v, null, 2); }}
                                    catch {{ return String(v); }}
                                }}
                                return String(v);
                            }}).join(' ');
                            globalThis.__tanaCore.ops.op_print_stderr(msg + "\n");
                        }}
                    }},
                }},
                version: {{
                    tana: "{tana_version}",
                    deno_core: "{deno_core_version}",
                    v8: "{v8_version}",
                }},
            }};

            // utils module
            tanaModules["tana/utils"] = {{
                async fetch(url) {{
                    if (!globalThis.__tanaCore) {{
                        throw new Error('Tana runtime not initialized');
                    }}
                    const result = await globalThis.__tanaCore.ops.op_fetch(url);
                    return {{
                        ok: true,
                        status: 200,
                        async text() {{ return result; }},
                        async json() {{ return JSON.parse(result); }}
                    }};
                }}
            }};

            // data module
            tanaModules["tana/data"] = {{
                data: {{
                    MAX_KEY_SIZE: 256,
                    MAX_VALUE_SIZE: 10240,
                    MAX_TOTAL_SIZE: 102400,
                    MAX_KEYS: 1000,

                    _serialize(value) {{
                        if (typeof value === 'string') return value;
                        return JSON.stringify(value, (key, val) => {{
                            if (typeof val === 'bigint') return val.toString();
                            return val;
                        }});
                    }},

                    _deserialize(value) {{
                        if (value === null) return null;
                        try {{ return JSON.parse(value); }}
                        catch {{ return value; }}
                    }},

                    async set(key, value) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        const serialized = this._serialize(value);
                        globalThis.__tanaCore.ops.op_data_set(key, serialized);
                    }},

                    async get(key) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        const value = globalThis.__tanaCore.ops.op_data_get(key);
                        return this._deserialize(value);
                    }},

                    async delete(key) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        globalThis.__tanaCore.ops.op_data_delete(key);
                    }},

                    async has(key) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_data_has(key);
                    }},

                    async keys(pattern) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_data_keys(pattern || null);
                    }},

                    async entries() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        const allKeys = await this.keys();
                        const result = {{}};
                        for (const key of allKeys) result[key] = await this.get(key);
                        return result;
                    }},

                    async clear() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        globalThis.__tanaCore.ops.op_data_clear();
                    }},

                    async commit() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        globalThis.__tanaCore.ops.op_data_commit();
                    }}
                }}
            }};

            // block module
            tanaModules["tana/block"] = {{
                block: {{
                    get height() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_height();
                    }},
                    get timestamp() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_timestamp();
                    }},
                    get hash() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_hash();
                    }},
                    get previousHash() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_previous_hash();
                    }},
                    get executor() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_executor();
                    }},
                    get contractId() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_contract_id();
                    }},
                    get gasLimit() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_gas_limit();
                    }},
                    get gasUsed() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_block_get_gas_used();
                    }},

                    MAX_BATCH_QUERY: 10,

                    async getBalance(userIds, currencyCode) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return await globalThis.__tanaCore.ops.op_block_get_balance(userIds, currencyCode);
                    }},

                    async getUser(userIds) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return await globalThis.__tanaCore.ops.op_block_get_user(userIds);
                    }},

                    async getTransaction(txIds) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return await globalThis.__tanaCore.ops.op_block_get_transaction(txIds);
                    }}
                }}
            }};

            // tx module
            tanaModules["tana/tx"] = {{
                tx: {{
                    transfer(from, to, amount, currency) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        globalThis.__tanaCore.ops.op_tx_transfer(from, to, amount, currency);
                    }},

                    setBalance(userId, amount, currency) {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        globalThis.__tanaCore.ops.op_tx_set_balance(userId, amount, currency);
                    }},

                    getChanges() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_tx_get_changes();
                    }},

                    async execute() {{
                        if (!globalThis.__tanaCore) throw new Error('Tana runtime not initialized');
                        return globalThis.__tanaCore.ops.op_tx_execute();
                    }}
                }}
            }};

            // Import shim
            globalThis.__tanaImport = function (spec) {{
                const m = tanaModules[spec];
                if (!m) throw new Error("unknown tana module: " + spec);
                return m;
            }};
            "#,
            tana_version = tana_version,
            deno_core_version = deno_core_version,
            v8_version = v8_version,
        );

        runtime
            .execute_script("simple-bootstrap.js", ModuleCodeString::from(simple_bootstrap))
            .expect("bootstrap lightweight");
    }

    eprintln!("  [TIMING] Bootstrap: {}ms", bootstrap_start.elapsed().as_millis());

    // 5) load and execute contract
    let exec_start = std::time::Instant::now();
    let user_code = fs::read_to_string(&file_path)
        .expect(&format!("failed to read contract: {}", file_path));

    if !is_precompiled {
        // Transpile TypeScript contract
        let runner = format!(
            r#"
            let src = {user_src};

            // line-by-line import rewriter so we don't clobber the whole file
            src = src
              .split("\n")
              .map((line) => {{
                const m = line.match(/^\s*import\s+{{([^}}]+)}}\s+from\s+["'](tana\/[^"']+)["'];?\s*$/);
                if (!m) return line;
                const names = m[1].trim();
                const spec = m[2].trim();
                return "const {{" + names + "}} = __tanaImport('" + spec + "');";
              }})
              .join("\n");

            const out = ts.transpileModule(src, {{
              compilerOptions: {{
                target: "ES2020",
                module: ts.ModuleKind.ESNext
              }}
            }});

            // Wrap in async IIFE to support top-level await (same as playground)
            const wrappedCode = "(async function() {{\n  'use strict';\n  " + out.outputText + "\n}})();";

            (0, eval)(wrappedCode);
            "#,
            user_src = serde_json::to_string(&user_code).unwrap(),
        );

        runtime
            .execute_script("run-user.ts", ModuleCodeString::from(runner))
            .expect("run user script");
    } else {
        // Execute pre-compiled JavaScript directly
        let runner = format!(
            r#"
            let src = {user_src};

            // Rewrite imports to use __tanaImport
            src = src
              .split("\n")
              .map((line) => {{
                const m = line.match(/^\s*import\s+{{([^}}]+)}}\s+from\s+["'](tana\/[^"']+)["'];?\s*$/);
                if (!m) return line;
                const names = m[1].trim();
                const spec = m[2].trim();
                return "const {{" + names + "}} = __tanaImport('" + spec + "');";
              }})
              .join("\n");

            // Wrap in async IIFE to support top-level await
            const wrappedCode = "(async function() {{\n  'use strict';\n  " + src + "\n}})();";

            (0, eval)(wrappedCode);
            "#,
            user_src = serde_json::to_string(&user_code).unwrap(),
        );

        runtime
            .execute_script("run-user.js", ModuleCodeString::from(runner))
            .expect("run user script");
    }

    eprintln!("  [TIMING] Contract execution: {}ms", exec_start.elapsed().as_millis());

    // Drive the event loop to completion (handles async ops like fetch)
    let event_loop_start = std::time::Instant::now();
    runtime
        .run_event_loop(deno_core::PollEventLoopOptions::default())
        .await
        .expect("event loop failed");
    eprintln!("  [TIMING] Event loop: {}ms", event_loop_start.elapsed().as_millis());

    eprintln!("\n  [TIMING] ═══ TOTAL TIME: {}ms ═══\n", total_start.elapsed().as_millis());
}