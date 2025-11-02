// tana-globals.ts
// internal API – user gets this in every execution

// Note: __tanaCore and Deno deletion are now handled in the Rust bootstrap
// This file just defines the user-facing `tana` global

const corePrint = (v: unknown) => {
  // @ts-ignore - __tanaCore is defined in bootstrap
  if (globalThis.__tanaCore) {
    // @ts-ignore
    globalThis.__tanaCore.print(String(v) + "\n");
  }
};

// Attach to globalThis - type definitions are in types/tana.d.ts
// @ts-ignore
globalThis.tana = {
  print: corePrint,
  version: "0.0.1",
};