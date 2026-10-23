//! wasm-bindgen surface over `calc_core` for the web front end.

use wasm_bindgen::prelude::*;

/// Evaluates an expression, returning the formatted result or throwing a JS
/// `Error` with a readable message.
#[wasm_bindgen]
pub fn evaluate(expression: &str) -> Result<String, JsError> {
    calc_core::evaluate(expression)
        .map(calc_core::format_number)
        .map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub struct Calculator {
    inner: calc_core::Calculator,
}

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Calculator {
        Calculator { inner: calc_core::Calculator::new() }
    }

    pub fn evaluate(&mut self, expression: &str) -> Result<String, JsError> {
        self.inner.evaluate(expression).map_err(|e| JsError::new(&e.to_string()))
    }

    /// History, oldest first.
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.inner
            .history()
            .iter()
            .map(|e| HistoryEntry { expression: e.expression.clone(), result: e.result.clone() })
            .collect()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

/// One history row. Exported as a JS class with getters, which avoids pulling
/// in js-sys/serde just to return structured data.
#[wasm_bindgen(getter_with_clone)]
pub struct HistoryEntry {
    pub expression: String,
    pub result: String,
}
