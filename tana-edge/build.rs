// build.rs
use std::fs;

fn main() {
    // super simple: read Cargo.lock as text
    let lock = fs::read_to_string("Cargo.lock").expect("Cargo.lock not found");

    // try to find the deno_core package line
    let deno_core_ver = lock
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("name = \"deno_core\""))
        .nth(1) // the next line is version = "..."
        .and_then(|l| l.trim_start().strip_prefix("version = \""))
        .and_then(|l| l.strip_suffix('"'))
        .unwrap_or("unknown");

    // try to find the v8 package line
    let v8_ver = lock
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("name = \"v8\""))
        .nth(1)
        .and_then(|l| l.trim_start().strip_prefix("version = \""))
        .and_then(|l| l.strip_suffix('"'))
        .unwrap_or("unknown");

    // pass to rustc
    println!("cargo:rustc-env=DENO_CORE_VERSION={}", deno_core_ver);
    println!("cargo:rustc-env=V8_VERSION={}", v8_ver);
}