use std::fs;
use std::sync::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::env;

use axum::{
    extract::Path as AxumPath,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
    http::StatusCode,
};
use tower_http::cors::CorsLayer;

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
const MOCK_EXECUTOR: &str = "user_edge_server";
const MOCK_CONTRACT_ID: &str = "contract_edge";
const MOCK_GAS_LIMIT: u64 = 1_000_000;
static MOCK_GAS_USED: Mutex<u64> = Mutex::new(0);

// Query limits (anti-abuse)
const MAX_BATCH_QUERY: usize = 10;

// ========== Ops (same as runtime) ==========

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

// ========== Blockchain State Query Ops (kept for compatibility) ==========

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

    // Fetch from ledger API (using TANA_LEDGER_URL env var or default to localhost)
    let ledger_url = env::var("TANA_LEDGER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let url = format!("{}/balances", ledger_url);
    let response = reqwest::get(&url).await
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

    // Fetch from ledger API (using TANA_LEDGER_URL env var or default to localhost)
    let ledger_url = env::var("TANA_LEDGER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let url = format!("{}/users", ledger_url);
    let response = reqwest::get(&url).await
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

    // Fetch from ledger API (using TANA_LEDGER_URL env var or default to localhost)
    let ledger_url = env::var("TANA_LEDGER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let url = format!("{}/transactions", ledger_url);
    let response = reqwest::get(&url).await
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

// ========== HTTP Handlers ==========

async fn handle_get(
    AxumPath(contract_id): AxumPath<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let start = std::time::Instant::now();
    let contract_id_for_log = contract_id.clone();
    eprintln!("[GET] Contract: {}", contract_id);

    let response = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            match execute_contract(&contract_id, "get").await {
                Ok(data) => data,
                Err(e) => serde_json::json!({ "status": 500, "body": { "error": e } }),
            }
        })
    })
    .await
    .unwrap_or_else(|e| serde_json::json!({ "status": 500, "body": { "error": format!("Task failed: {}", e) } }));

    // Extract status code from response
    let status_code = response.get("status")
        .and_then(|s| s.as_u64())
        .and_then(|s| StatusCode::from_u16(s as u16).ok())
        .unwrap_or(StatusCode::OK);

    let duration = start.elapsed();
    println!(
        "[METRICS] method=GET contract={} status={} duration={}ms",
        contract_id_for_log,
        status_code.as_u16(),
        duration.as_millis()
    );

    (status_code, Json(response))
}

async fn handle_post(
    AxumPath(contract_id): AxumPath<String>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let start = std::time::Instant::now();
    let contract_id_for_log = contract_id.clone();
    eprintln!("[POST] Contract: {}, Body: {:?}", contract_id, body);

    let response = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            match execute_contract_with_body(&contract_id, "post", body).await {
                Ok(data) => data,
                Err(e) => serde_json::json!({ "status": 500, "body": { "error": e } }),
            }
        })
    })
    .await
    .unwrap_or_else(|e| serde_json::json!({ "status": 500, "body": { "error": format!("Task failed: {}", e) } }));

    // Extract status code from response
    let status_code = response.get("status")
        .and_then(|s| s.as_u64())
        .and_then(|s| StatusCode::from_u16(s as u16).ok())
        .unwrap_or(StatusCode::OK);

    let duration = start.elapsed();
    println!(
        "[METRICS] method=POST contract={} status={} duration={}ms",
        contract_id_for_log,
        status_code.as_u16(),
        duration.as_millis()
    );

    (status_code, Json(response))
}

// Execute a contract and return JSON response
async fn execute_contract(
    contract_id: &str,
    method: &str,
) -> Result<serde_json::Value, String> {
    execute_contract_with_body(contract_id, method, serde_json::json!({})).await
}

// Execute a contract with POST body
async fn execute_contract_with_body(
    contract_id: &str,
    method: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Construct paths for both .js (pre-compiled) and .ts (source)
    // Try ./contracts first (running from project root), then ../contracts (running from tana-edge/)
    let contract_dir = if PathBuf::from("./contracts").join(contract_id).exists() {
        PathBuf::from("./contracts").join(contract_id)
    } else {
        PathBuf::from("../contracts").join(contract_id)
    };
    let js_path = contract_dir.join(format!("{}.js", method));
    let ts_path = contract_dir.join(format!("{}.ts", method));

    // Prefer pre-compiled .js if it exists
    let (contract_path, is_precompiled) = if js_path.exists() {
        eprintln!("[EXEC] Using pre-compiled: {}", js_path.display());
        (js_path, true)
    } else if ts_path.exists() {
        eprintln!("[EXEC] Using TypeScript: {}", ts_path.display());
        (ts_path, false)
    } else {
        return Err(format!("Contract not found: {}", contract_dir.join(method).display()));
    };

    // Read contract source
    let contract_source = tokio::fs::read_to_string(&contract_path)
        .await
        .map_err(|e| format!("Failed to read contract: {}", e))?;

    eprintln!("[EXEC] Contract loaded, executing...");

    // Execute contract in V8 runtime
    let result = run_contract(&contract_source, is_precompiled, body).await?;

    Ok(result)
}

// Run contract code in V8 runtime and capture return value
async fn run_contract(
    contract_source: &str,
    is_precompiled: bool,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let total_start = std::time::Instant::now();
    eprintln!("  [INIT] Pre-compiled: {}", is_precompiled);

    // Create extension with all ops
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
    const OP_BLOCK_GET_HEIGHT: deno_core::OpDecl = op_block_get_height();
    const OP_BLOCK_GET_TIMESTAMP: deno_core::OpDecl = op_block_get_timestamp();
    const OP_BLOCK_GET_HASH: deno_core::OpDecl = op_block_get_hash();
    const OP_BLOCK_GET_PREVIOUS_HASH: deno_core::OpDecl = op_block_get_previous_hash();
    const OP_BLOCK_GET_EXECUTOR: deno_core::OpDecl = op_block_get_executor();
    const OP_BLOCK_GET_CONTRACT_ID: deno_core::OpDecl = op_block_get_contract_id();
    const OP_BLOCK_GET_GAS_LIMIT: deno_core::OpDecl = op_block_get_gas_limit();
    const OP_BLOCK_GET_GAS_USED: deno_core::OpDecl = op_block_get_gas_used();
    const OP_BLOCK_GET_BALANCE: deno_core::OpDecl = op_block_get_balance();
    const OP_BLOCK_GET_USER: deno_core::OpDecl = op_block_get_user();
    const OP_BLOCK_GET_TRANSACTION: deno_core::OpDecl = op_block_get_transaction();
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

    // Create runtime
    let runtime_start = std::time::Instant::now();
    let mut runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![ext],
        module_loader: None,
        ..Default::default()
    });
    eprintln!("  [TIMING] V8 runtime creation: {}ms", runtime_start.elapsed().as_millis());

    // Load TypeScript compiler (only if not pre-compiled)
    if !is_precompiled {
        let ts_load_start = std::time::Instant::now();
        // Try ./typescript.js first (running from tana-edge/), then tana-edge/typescript.js (running from project root)
        let ts_path = if PathBuf::from("./typescript.js").exists() {
            "./typescript.js"
        } else {
            "tana-edge/typescript.js"
        };
        let ts_src = fs::read_to_string(ts_path)
            .map_err(|e| format!("Missing typescript.js: {}", e))?;
        runtime
            .execute_script("typescript.js", ModuleCodeString::from(ts_src))
            .map_err(|e| format!("Failed to load TypeScript: {}", e))?;
        eprintln!("  [TIMING] TypeScript compiler load: {}ms", ts_load_start.elapsed().as_millis());
    } else {
        eprintln!("  [TIMING] TypeScript compiler load: 0ms (pre-compiled JS)");
    }

    // Load tana globals
    // Try ./tana-globals.ts first (running from tana-edge/), then tana-edge/tana-globals.ts (running from project root)
    let globals_path = if PathBuf::from("./tana-globals.ts").exists() {
        "./tana-globals.ts"
    } else {
        "tana-edge/tana-globals.ts"
    };
    let tana_globals = fs::read_to_string(globals_path)
        .map_err(|e| format!("Missing tana-globals.ts: {}", e))?;

    let tana_version = env!("CARGO_PKG_VERSION");
    let deno_core_version = env!("DENO_CORE_VERSION");
    let v8_version = env!("V8_VERSION");

    // Bootstrap globals (with tana/net module added)
    let bootstrap_globals = format!(
        r#"
        globalThis.__tanaCore = globalThis.Deno?.core;
        delete globalThis.Deno;

        const tanaModules = Object.create(null);

        // tana/core module
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

        // tana/net module (NEW - for edge requests/responses)
        tanaModules["tana/net"] = {{
            Request: class Request {{
                constructor(data) {{
                    this.path = data?.path || '/';
                    this.method = data?.method || 'GET';
                    this.query = data?.query || {{}};
                    this.headers = data?.headers || {{}};
                    this.params = data?.params || {{}};
                    this.ip = data?.ip || '127.0.0.1';
                }}
            }},
            Response: class Response {{
                constructor(status, body, headers) {{
                    this.status = status || 200;
                    this.body = body || null;
                    this.headers = headers || {{}};
                }}

                static json(data, status = 200) {{
                    return new Response(status, data, {{ 'Content-Type': 'application/json' }});
                }}

                static text(data, status = 200) {{
                    return new Response(status, data, {{ 'Content-Type': 'text/plain' }});
                }}
            }}
        }};

        // tana/block module (blockchain queries)
        tanaModules["tana/block"] = {{
            block: {{
                async getBalance(userIds, currencyCode) {{
                    return globalThis.__tanaCore.ops.op_block_get_balance(userIds, currencyCode);
                }},
                async getUser(userIds) {{
                    return globalThis.__tanaCore.ops.op_block_get_user(userIds);
                }},
                async getTransaction(txIds) {{
                    return globalThis.__tanaCore.ops.op_block_get_transaction(txIds);
                }},
                getHeight() {{
                    return globalThis.__tanaCore.ops.op_block_get_height();
                }},
                getTimestamp() {{
                    return globalThis.__tanaCore.ops.op_block_get_timestamp();
                }},
                getHash() {{
                    return globalThis.__tanaCore.ops.op_block_get_hash();
                }},
                getPreviousHash() {{
                    return globalThis.__tanaCore.ops.op_block_get_previous_hash();
                }},
                getExecutor() {{
                    return globalThis.__tanaCore.ops.op_block_get_executor();
                }},
                getContractId() {{
                    return globalThis.__tanaCore.ops.op_block_get_contract_id();
                }},
                getGasLimit() {{
                    return globalThis.__tanaCore.ops.op_block_get_gas_limit();
                }},
                getGasUsed() {{
                    return globalThis.__tanaCore.ops.op_block_get_gas_used();
                }},
            }}
        }};

        // tana/tx module (transaction staging)
        tanaModules["tana/tx"] = {{
            tx: {{
                transfer(from, to, amount, currency) {{
                    globalThis.__tanaCore.ops.op_tx_transfer(from, to, amount, currency);
                }},
                setBalance(userId, amount, currency) {{
                    globalThis.__tanaCore.ops.op_tx_set_balance(userId, amount, currency);
                }},
                getChanges() {{
                    return globalThis.__tanaCore.ops.op_tx_get_changes();
                }},
                execute() {{
                    return globalThis.__tanaCore.ops.op_tx_execute();
                }},
            }}
        }};

        // tana/utils module (external fetch)
        tanaModules["tana/utils"] = {{
            async fetch(url) {{
                const response = await globalThis.__tanaCore.ops.op_fetch(url);
                return {{
                    async json() {{
                        return JSON.parse(response);
                    }},
                    async text() {{
                        return response;
                    }},
                }};
            }}
        }};

        // tana/data module (key-value storage)
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
                    const serialized = this._serialize(value);
                    globalThis.__tanaCore.ops.op_data_set(key, serialized);
                }},
                async get(key) {{
                    const value = globalThis.__tanaCore.ops.op_data_get(key);
                    return this._deserialize(value);
                }},
                async delete(key) {{
                    globalThis.__tanaCore.ops.op_data_delete(key);
                }},
                async has(key) {{
                    return globalThis.__tanaCore.ops.op_data_has(key);
                }},
                async keys(pattern) {{
                    return globalThis.__tanaCore.ops.op_data_keys(pattern || null);
                }},
                async entries() {{
                    const allKeys = await this.keys();
                    const result = {{}};
                    for (const key of allKeys) {{
                        result[key] = await this.get(key);
                    }}
                    return result;
                }},
                async clear() {{
                    globalThis.__tanaCore.ops.op_data_clear();
                }},
                async commit() {{
                    globalThis.__tanaCore.ops.op_data_commit();
                }}
            }}
        }};

        // Load user-defined globals
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

        // Import shim
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

    // Bootstrap globals
    let bootstrap_start = std::time::Instant::now();
    if !is_precompiled {
        // Full bootstrap with TypeScript transpilation
        runtime
            .execute_script("tana-bootstrap.js", ModuleCodeString::from(bootstrap_globals))
            .map_err(|e| format!("Failed to bootstrap: {}", e))?;
    } else {
        // Lightweight bootstrap for pre-compiled JS (skip tana-globals transpilation)
        let simple_bootstrap = r#"
        globalThis.__tanaCore = globalThis.Deno?.core;
        delete globalThis.Deno;

        const tanaModules = Object.create(null);

        // All the tana modules are already set up in tana-globals which was transpiled
        // We just need the import shim
        globalThis.__tanaImport = function (spec) {
          const m = tanaModules[spec];
          if (!m) throw new Error("unknown tana module: " + spec);
          return m;
        };

        // Set up basic modules that don't need user-defined globals
        tanaModules["tana/core"] = {
            console: {
                log(...args) {
                    if (globalThis.__tanaCore) {
                        const msg = args.map(v => {
                            if (typeof v === 'object') {
                                try { return JSON.stringify(v, null, 2); }
                                catch { return String(v); }
                            }
                            return String(v);
                        }).join(' ');
                        globalThis.__tanaCore.print(msg + "\n");
                    }
                },
                error(...args) {
                    if (globalThis.__tanaCore) {
                        const msg = args.map(v => {
                            if (typeof v === 'object') {
                                try { return JSON.stringify(v, null, 2); }
                                catch { return String(v); }
                            }
                            return String(v);
                        }).join(' ');
                        globalThis.__tanaCore.ops.op_print_stderr(msg + "\n");
                    }
                },
            },
            version: {
                tana: "0.1.0",
            },
        };

        tanaModules["tana/net"] = {
            Request: class Request {
                constructor(data) {
                    this.path = data?.path || '/';
                    this.method = data?.method || 'GET';
                    this.query = data?.query || {};
                    this.headers = data?.headers || {};
                    this.params = data?.params || {};
                    this.ip = data?.ip || '127.0.0.1';
                }
            },
            Response: class Response {
                constructor(status, body, headers) {
                    this.status = status || 200;
                    this.body = body || null;
                    this.headers = headers || {};
                }

                static json(data, status = 200) {
                    return new Response(status, data, { 'Content-Type': 'application/json' });
                }

                static text(data, status = 200) {
                    return new Response(status, data, { 'Content-Type': 'text/plain' });
                }
            }
        };

        tanaModules["tana/block"] = {
            block: {
                async getBalance(userIds, currencyCode) {
                    return globalThis.__tanaCore.ops.op_block_get_balance(userIds, currencyCode);
                },
                async getUser(userIds) {
                    return globalThis.__tanaCore.ops.op_block_get_user(userIds);
                },
                async getTransaction(txIds) {
                    return globalThis.__tanaCore.ops.op_block_get_transaction(txIds);
                },
                getHeight() {
                    return globalThis.__tanaCore.ops.op_block_get_height();
                },
                getTimestamp() {
                    return globalThis.__tanaCore.ops.op_block_get_timestamp();
                },
                getHash() {
                    return globalThis.__tanaCore.ops.op_block_get_hash();
                },
                getPreviousHash() {
                    return globalThis.__tanaCore.ops.op_block_get_previous_hash();
                },
                getExecutor() {
                    return globalThis.__tanaCore.ops.op_block_get_executor();
                },
                getContractId() {
                    return globalThis.__tanaCore.ops.op_block_get_contract_id();
                },
                getGasLimit() {
                    return globalThis.__tanaCore.ops.op_block_get_gas_limit();
                },
                getGasUsed() {
                    return globalThis.__tanaCore.ops.op_block_get_gas_used();
                },
            }
        };

        tanaModules["tana/tx"] = {
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
                execute() {
                    return globalThis.__tanaCore.ops.op_tx_execute();
                },
            }
        };

        tanaModules["tana/utils"] = {
            async fetch(url) {
                const response = await globalThis.__tanaCore.ops.op_fetch(url);
                return {
                    async json() {
                        return JSON.parse(response);
                    },
                    async text() {
                        return response;
                    },
                };
            }
        };

        tanaModules["tana/data"] = {
            data: {
                MAX_KEY_SIZE: 256,
                MAX_VALUE_SIZE: 10240,
                MAX_TOTAL_SIZE: 102400,
                MAX_KEYS: 1000,
                _serialize(value) {
                    if (typeof value === 'string') return value;
                    return JSON.stringify(value, (key, val) => {
                        if (typeof val === 'bigint') return val.toString();
                        return val;
                    });
                },
                _deserialize(value) {
                    if (value === null) return null;
                    try { return JSON.parse(value); }
                    catch { return value; }
                },
                async set(key, value) {
                    const serialized = this._serialize(value);
                    globalThis.__tanaCore.ops.op_data_set(key, serialized);
                },
                async get(key) {
                    const value = globalThis.__tanaCore.ops.op_data_get(key);
                    return this._deserialize(value);
                },
                async delete(key) {
                    globalThis.__tanaCore.ops.op_data_delete(key);
                },
                async has(key) {
                    return globalThis.__tanaCore.ops.op_data_has(key);
                },
                async keys(pattern) {
                    return globalThis.__tanaCore.ops.op_data_keys(pattern || null);
                },
                async entries() {
                    const allKeys = await this.keys();
                    const result = {};
                    for (const key of allKeys) {
                        result[key] = await this.get(key);
                    }
                    return result;
                },
                async clear() {
                    globalThis.__tanaCore.ops.op_data_clear();
                },
                async commit() {
                    globalThis.__tanaCore.ops.op_data_commit();
                }
            }
        };
        "#;
        runtime
            .execute_script("simple-bootstrap.js", ModuleCodeString::from(simple_bootstrap.to_string()))
            .map_err(|e| format!("Failed to bootstrap: {}", e))?;
    }
    eprintln!("  [TIMING] Bootstrap globals: {}ms", bootstrap_start.elapsed().as_millis());

    // Execute contract code and capture return value
    let contract_start = std::time::Instant::now();
    let runner = if is_precompiled {
        // Pre-compiled JS - skip transpilation, just execute
        format!(
            r#"
            let contractSrc = {contract_src};

            // Rewrite imports (still needed even for pre-compiled JS)
            contractSrc = contractSrc
              .split("\n")
              .map((line) => {{
                const importMatch = line.match(/^\s*import\s+{{([^}}]+)}}\s+from\s+["'](tana\/[^"']+)["'];?\s*$/);
                if (importMatch) {{
                  const names = importMatch[1].trim();
                  const spec = importMatch[2].trim();
                  return "const {{" + names + "}} = __tanaImport('" + spec + "');";
                }}
                return line.replace(/^(\s*)export\s+/, '$1');
              }})
              .join("\n");

            // Execute pre-compiled JS directly (no transpilation!)
            let __contractResult;
            (async function() {{
              'use strict';
              const module = {{}};
              const exports = {{}};
              module.exports = exports;

              (0, eval)(contractSrc);

              if (typeof Get === 'function') {{
                const req = new (__tanaImport('tana/net').Request)({{
                  path: '/',
                  method: 'GET'
                }});
                __contractResult = await Get(req);
              }} else if (typeof Post === 'function') {{
                const req = new (__tanaImport('tana/net').Request)({{
                  path: '/',
                  method: 'POST'
                }});
                __contractResult = await Post(req, {post_body});
              }} else {{
                __contractResult = {{ status: 500, body: {{ error: "No Get or Post function exported" }} }};
              }}
            }})();
            "#,
            contract_src = serde_json::to_string(&contract_source).unwrap(),
            post_body = serde_json::to_string(&body).unwrap(),
        )
    } else {
        // TypeScript - needs transpilation
        format!(
            r#"
            let contractSrc = {contract_src};

            // Rewrite imports and exports
            contractSrc = contractSrc
              .split("\n")
              .map((line) => {{
                const importMatch = line.match(/^\s*import\s+{{([^}}]+)}}\s+from\s+["'](tana\/[^"']+)["'];?\s*$/);
                if (importMatch) {{
                  const names = importMatch[1].trim();
                  const spec = importMatch[2].trim();
                  return "const {{" + names + "}} = __tanaImport('" + spec + "');";
                }}
                return line.replace(/^(\s*)export\s+/, '$1');
              }})
              .join("\n");

            const out = ts.transpileModule(contractSrc, {{
              compilerOptions: {{
                target: "ES2020",
                module: ts.ModuleKind.ESNext
              }}
            }});

            let __contractResult;
            (async function() {{
              'use strict';
              const module = {{}};
              const exports = {{}};
              module.exports = exports;

              (0, eval)(out.outputText);

              if (typeof Get === 'function') {{
                const req = new (__tanaImport('tana/net').Request)({{
                  path: '/',
                  method: 'GET'
                }});
                __contractResult = await Get(req);
              }} else if (typeof Post === 'function') {{
                const req = new (__tanaImport('tana/net').Request)({{
                  path: '/',
                  method: 'POST'
                }});
                __contractResult = await Post(req, {post_body});
              }} else {{
                __contractResult = {{ status: 500, body: {{ error: "No Get or Post function exported" }} }};
              }}
            }})();
            "#,
            contract_src = serde_json::to_string(&contract_source).unwrap(),
            post_body = serde_json::to_string(&body).unwrap(),
        )
    };

    runtime
        .execute_script("run-contract.ts", ModuleCodeString::from(runner))
        .map_err(|e| format!("Failed to execute contract: {}", e))?;

    // Run event loop
    let event_loop_start = std::time::Instant::now();
    runtime
        .run_event_loop(deno_core::PollEventLoopOptions::default())
        .await
        .map_err(|e| format!("Event loop failed: {}", e))?;
    eprintln!("  [TIMING] Contract execution + event loop: {}ms", event_loop_start.elapsed().as_millis());

    eprintln!("  [TIMING] Total contract execution: {}ms", contract_start.elapsed().as_millis());

    // Get the result from global scope
    let result_start = std::time::Instant::now();
    let get_result = r#"
        JSON.stringify(__contractResult || { status: 500, body: { error: "No result returned" } })
    "#;

    let result_value = runtime
        .execute_script("get-result", ModuleCodeString::from(get_result.to_string()))
        .map_err(|e| format!("Failed to get result: {}", e))?;

    // Convert to JSON
    let scope = &mut runtime.handle_scope();
    let local = deno_core::v8::Local::new(scope, result_value);
    let result_str = local.to_rust_string_lossy(scope);

    let result = serde_json::from_str(&result_str)
        .map_err(|e| format!("Failed to parse result: {}", e))?;

    eprintln!("  [TIMING] Result extraction: {}ms", result_start.elapsed().as_millis());
    eprintln!("  [TIMING] ═══ TOTAL V8 TIME: {}ms ═══", total_start.elapsed().as_millis());

    Ok(result)
}

#[tokio::main]
async fn main() {
    eprintln!("🚀 Starting tana-edge server...");

    // Build router
    let app = Router::new()
        .route("/:contract_id", get(handle_get))
        .route("/:contract_id/*path", get(handle_get))
        .route("/:contract_id", post(handle_post))
        .route("/:contract_id/*path", post(handle_post))
        .layer(CorsLayer::permissive());

    // Start server
    let addr = "127.0.0.1:8180";
    eprintln!("✅ tana-edge is running on http://{}", addr);
    eprintln!("📝 contracts directory: ../contracts/");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
