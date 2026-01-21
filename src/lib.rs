// Library crate for the WASM web client.
//
// This is intentionally minimal so the workspace builds even if the WASM
// entrypoint is still being implemented.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    // Better panic messages in the browser console.
    console_error_panic_hook::set_once();
}

// Non-wasm builds (e.g. `cargo test` on x86_64) still need a symbol to satisfy
// the crate, but they don't do anything.
#[cfg(not(target_arch = "wasm32"))]
pub fn start() {}
