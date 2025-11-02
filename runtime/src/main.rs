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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 1) expose our ops
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
        ]),
        ..Default::default()
    };

    // 2) runtime – NO custom module loader for now
    let mut runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![ext],
        // we'll just use the default loader (= scripts only)
        module_loader: None,
        ..Default::default()
    });

    // 3) load TS compiler (downloaded once next to the binary)
    let ts_src = fs::read_to_string("typescript.js")
        .expect("missing typescript.js");
    runtime
        .execute_script("typescript.js", ModuleCodeString::from(ts_src))
        .expect("load ts");

    // 4) load our internal globals (your tana-globals.ts)
    let tana_globals = fs::read_to_string("tana-globals.ts")
        .expect("missing tana-globals.ts");

    // derive our own crate version from Cargo
    let tana_version = env!("CARGO_PKG_VERSION");
    // for now we can't query deno_core/v8 at runtime in this version,
    // so keep them as compile-time strings (can be filled by build.rs later)
    let deno_core_version = env!("DENO_CORE_VERSION");
    let v8_version = env!("V8_VERSION");

    // this shim gives us a "fake" module system in JS:
    //   import { console } from "tana:core"
    // will become a lookup into a JS map.
    let bootstrap_globals = format!(
        r#"
        // 1. FIRST: Stash Deno.core before we delete it
        globalThis.__tanaCore = globalThis.Deno?.core;

        // 2. Delete Deno to create sandbox
        delete globalThis.Deno;

        // 3. NOW we can safely define modules that use __tanaCore
        const tanaModules = Object.create(null);

        // core module - browser-like console API
        tanaModules["tana:core"] = {{
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
        tanaModules["tana:utils"] = {{
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
        tanaModules["tana:data"] = {{
            data: {{
                MAX_KEY_SIZE: 256,
                MAX_VALUE_SIZE: 10240,
                MAX_TOTAL_SIZE: 102400,
                MAX_KEYS: 1000,

                // Helper: serialize value (supports strings and objects)
                _serialize(value) {{
                    if (typeof value === 'string') {{
                        return value;
                    }}
                    return JSON.stringify(value);
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

    // 5) load user TS
    let user_ts = fs::read_to_string("example.ts")
        .expect("missing example.ts");

    // 6) transpile+run user TS, but rewrite `import ... from "tana:*"`
    //    into calls to __tanaImport so we don't need Rust ModuleLoader.
    let runner = format!(
        r#"
        let src = {user_src};

        // line-by-line import rewriter so we don't clobber the whole file
        src = src
          .split("\n")
          .map((line) => {{
            const m = line.match(/^\s*import\s+{{([^}}]+)}}\s+from\s+["'](tana:[^"']+)["'];?\s*$/);
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
        user_src = serde_json::to_string(&user_ts).unwrap(),
    );

    runtime
        .execute_script("run-user.ts", ModuleCodeString::from(runner))
        .expect("run user script");

    // Drive the event loop to completion (handles async ops like fetch)
    runtime
        .run_event_loop(deno_core::PollEventLoopOptions::default())
        .await
        .expect("event loop failed");
}