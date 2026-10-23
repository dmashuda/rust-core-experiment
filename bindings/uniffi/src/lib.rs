//! UniFFI surface over `calc_core` for Swift (Apple) and Kotlin (Android).
//!
//! Proc-macro mode: no .udl file and no build.rs. Bindings are generated
//! from the compiled library's embedded metadata (see //bindings/uniffi:BUILD).

use std::sync::Mutex;

uniffi::setup_scaffolding!();

/// Surfaces as a Swift `Error` / Kotlin exception.
#[derive(Debug, uniffi::Error)]
pub enum CalcError {
    Invalid { message: String },
}

impl std::error::Error for CalcError {}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalcError::Invalid { message } => f.write_str(message),
        }
    }
}

impl From<calc_core::CalcError> for CalcError {
    fn from(e: calc_core::CalcError) -> Self {
        CalcError::Invalid { message: e.to_string() }
    }
}

#[derive(uniffi::Record)]
pub struct HistoryEntry {
    pub expression: String,
    pub result: String,
}

/// Evaluates an expression and returns the formatted result.
#[uniffi::export]
pub fn evaluate(expression: String) -> Result<String, CalcError> {
    Ok(calc_core::format_number(calc_core::evaluate(&expression)?))
}

/// Stateful calculator. UniFFI objects are shared (`Arc`) across the FFI
/// boundary and may be called from any thread, hence the `Mutex`.
#[derive(uniffi::Object)]
pub struct Calculator {
    inner: Mutex<calc_core::Calculator>,
}

#[uniffi::export]
impl Calculator {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Calculator { inner: Mutex::new(calc_core::Calculator::new()) }
    }

    pub fn evaluate(&self, expression: String) -> Result<String, CalcError> {
        Ok(self.inner.lock().unwrap().evaluate(&expression)?)
    }

    /// History, oldest first.
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.inner
            .lock()
            .unwrap()
            .history()
            .iter()
            .map(|e| HistoryEntry { expression: e.expression.clone(), result: e.result.clone() })
            .collect()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}
