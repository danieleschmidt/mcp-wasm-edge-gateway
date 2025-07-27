//! WASM-specific utilities and bindings

use js_sys::Promise;
use wasm_bindgen::prelude::*;
use web_sys::{console, window, Window};

/// Initialize WASM module with logging
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log!("MCP WASM Edge Gateway initialized");
}

/// Log to browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Macro for console logging in WASM
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Get current timestamp in WASM environment
#[wasm_bindgen]
pub fn get_timestamp() -> f64 {
    js_sys::Date::now()
}

/// Sleep function for WASM
#[wasm_bindgen]
pub async fn sleep(ms: u32) -> Result<(), JsValue> {
    let promise = Promise::new(&mut |resolve, _| {
        let window = window().unwrap();
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap();
    });

    wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok(())
}

/// Get available memory in WASM environment
#[wasm_bindgen]
pub fn get_memory_usage() -> u32 {
    // In WASM, memory usage is managed by the runtime
    // This is a placeholder implementation
    if let Some(performance) = window().and_then(|w| w.performance()) {
        if let Ok(memory) = js_sys::Reflect::get(&performance, &JsValue::from_str("memory")) {
            if let Some(memory_obj) = memory.dyn_ref::<js_sys::Object>() {
                if let Ok(used) =
                    js_sys::Reflect::get(memory_obj, &JsValue::from_str("usedJSHeapSize"))
                {
                    return used.as_f64().unwrap_or(0.0) as u32 / 1024 / 1024; // Convert to MB
                }
            }
        }
    }
    0
}

/// Check if running in a web worker
#[wasm_bindgen]
pub fn is_web_worker() -> bool {
    window().is_none()
}

/// Get user agent string
#[wasm_bindgen]
pub fn get_user_agent() -> String {
    window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Simple local storage wrapper
#[wasm_bindgen]
pub struct LocalStorage;

#[wasm_bindgen]
impl LocalStorage {
    #[wasm_bindgen(constructor)]
    pub fn new() -> LocalStorage {
        LocalStorage
    }

    #[wasm_bindgen]
    pub fn set_item(&self, key: &str, value: &str) -> Result<(), JsValue> {
        window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap()
            .set_item(key, value)
    }

    #[wasm_bindgen]
    pub fn get_item(&self, key: &str) -> Option<String> {
        window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap()
            .get_item(key)
            .unwrap()
    }

    #[wasm_bindgen]
    pub fn remove_item(&self, key: &str) -> Result<(), JsValue> {
        window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap()
            .remove_item(key)
    }
}

/// WASM-specific error handling
#[wasm_bindgen]
pub fn handle_panic(info: &str) {
    console_log!("Panic occurred: {}", info);
}

/// Performance measurement utilities
#[wasm_bindgen]
pub struct PerformanceMeasure {
    start_time: f64,
    name: String,
}

#[wasm_bindgen]
impl PerformanceMeasure {
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str) -> PerformanceMeasure {
        let start_time = get_timestamp();
        console_log!("Starting measurement: {}", name);

        PerformanceMeasure {
            start_time,
            name: name.to_string(),
        }
    }

    #[wasm_bindgen]
    pub fn end(&self) -> f64 {
        let duration = get_timestamp() - self.start_time;
        console_log!("Measurement {} completed in {}ms", self.name, duration);
        duration
    }
}

/// Network utilities for WASM
#[wasm_bindgen]
pub async fn fetch_data(url: &str) -> Result<String, JsValue> {
    let window = window().unwrap();
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await?;
    let resp: web_sys::Response = resp_value.dyn_into().unwrap();
    let text = wasm_bindgen_futures::JsFuture::from(resp.text()?).await?;
    Ok(text.as_string().unwrap())
}

/// Check WebAssembly SIMD support
#[wasm_bindgen]
pub fn has_simd_support() -> bool {
    // This would need to be implemented with feature detection
    // For now, return false as a conservative default
    false
}

/// WebAssembly feature detection
#[wasm_bindgen]
pub struct WasmCapabilities {
    pub simd: bool,
    pub threads: bool,
    pub memory64: bool,
}

#[wasm_bindgen]
impl WasmCapabilities {
    #[wasm_bindgen(constructor)]
    pub fn detect() -> WasmCapabilities {
        WasmCapabilities {
            simd: has_simd_support(),
            threads: false,  // Most browsers don't support WASM threads yet
            memory64: false, // Memory64 is not widely supported
        }
    }
}
