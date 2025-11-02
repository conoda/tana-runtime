use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wee_alloc::WeeAlloc;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

use deno_core::op2;
use deno_core::{Extension, JsRuntime, ModuleCodeString, RuntimeOptions};

// Output capture for WASM
thread_local! {
    static OUTPUT: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static ERRORS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

#[op2(fast)]
fn op_print_stdout(#[string] msg: String) {
    OUTPUT.with(|output| {
        output.borrow_mut().push(msg);
    });
}

#[op2(fast)]
fn op_print_stderr(#[string] msg: String) {
    ERRORS.with(|errors| {
        errors.borrow_mut().push(msg);
    });
}

#[op2]
fn op_sum(#[serde] nums: Vec<f64>) -> Result<f64, deno_error::JsErrorBox> {
    Ok(nums.iter().sum())
}

#[wasm_bindgen]
pub struct TanaRuntime {
    runtime: JsRuntime,
    typescript_loaded: bool,
}

#[wasm_bindgen]
impl TanaRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<TanaRuntime, JsValue> {
        #[cfg(target_arch = "wasm32")]
        console_error_panic_hook::set_once();

        // Set up extensions with our ops
        const OP_SUM: deno_core::OpDecl = op_sum();
        const OP_PRINT_STDOUT: deno_core::OpDecl = op_print_stdout();
        const OP_PRINT_STDERR: deno_core::OpDecl = op_print_stderr();

        let ext = Extension {
            name: "tana_ext",
            ops: std::borrow::Cow::Borrowed(&[OP_SUM, OP_PRINT_STDOUT, OP_PRINT_STDERR]),
            ..Default::default()
        };

        let runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![ext],
            module_loader: None,
            ..Default::default()
        });

        Ok(TanaRuntime {
            runtime,
            typescript_loaded: false,
        })
    }

    #[wasm_bindgen]
    pub fn load_typescript(&mut self, ts_source: &str) -> Result<(), JsValue> {
        self.runtime
            .execute_script("typescript.js", ModuleCodeString::from(ts_source.to_string()))
            .map_err(|e| JsValue::from_str(&format!("Failed to load TypeScript: {:?}", e)))?;

        self.typescript_loaded = true;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn bootstrap(&mut self, tana_version: &str, deno_core_version: &str, v8_version: &str) -> Result<(), JsValue> {
        if !self.typescript_loaded {
            return Err(JsValue::from_str("TypeScript compiler not loaded. Call load_typescript() first."));
        }

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
                            globalThis.__tanaCore.ops.op_print_stdout(msg + "\n");
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

        self.runtime
            .execute_script("tana-bootstrap.js", ModuleCodeString::from(bootstrap_globals))
            .map_err(|e| JsValue::from_str(&format!("Bootstrap failed: {:?}", e)))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, user_code: &str) -> Result<String, JsValue> {
        // Clear previous output
        OUTPUT.with(|o| o.borrow_mut().clear());
        ERRORS.with(|e| e.borrow_mut().clear());

        let runner = format!(
            r#"
            let src = {user_src};

            // line-by-line import rewriter
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

            (0, eval)(out.outputText);
            "#,
            user_src = serde_json::to_string(user_code).unwrap(),
        );

        self.runtime
            .execute_script("run-user.ts", ModuleCodeString::from(runner))
            .map_err(|e| JsValue::from_str(&format!("Execution error: {:?}", e)))?;

        // Collect output
        let stdout = OUTPUT.with(|o| o.borrow().join(""));
        let stderr = ERRORS.with(|e| e.borrow().join(""));

        let result = if stderr.is_empty() {
            stdout
        } else {
            format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr)
        };

        Ok(result)
    }
}
