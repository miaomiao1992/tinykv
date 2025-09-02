//! WASM-specific bindings and utilities for TinyKV

#[cfg(not(feature = "std"))]
use alloc::string::String;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // LocalStorage bindings
    #[wasm_bindgen(js_namespace = localStorage, js_name = getItem)]
    pub fn ls_get_item(key: &str) -> Option<String>;

    #[wasm_bindgen(js_namespace = localStorage, js_name = setItem)]
    pub fn ls_set_item(key: &str, value: &str);

    #[wasm_bindgen(js_namespace = localStorage, js_name = removeItem)]
    pub fn ls_remove_item(key: &str);

    // Timestamp function
    #[wasm_bindgen(js_name = "Date.now")]
    pub fn date_now() -> f64;

    // Console logging for debugging
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

/// Returns the current timestamp in seconds since the UNIX epoch.
/// Helper function for WASM environments.
pub fn current_timestamp() -> u64 {
    (date_now() / 1000.0) as u64
}

/// Web storage backend types for WASM environments.
/// Supports both localStorage storage.
#[derive(Debug, Clone)]
pub enum WebStorageBackend {
    LocalStorage,
}
