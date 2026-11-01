//! wasm-bindgen surface over `calc_ui` for the web front end.
//!
//! Structs are exported as JS classes with getters (`getter_with_clone`).
//! wasm-bindgen frees them through a FinalizationRegistry once JS drops
//! them, so the renderer never has to call `.free()`.
//!
//! History is SQLite here too -- the same crate and schema the native apps
//! use, compiled to wasm -- but held in SQLite's own memory, so it lasts as
//! long as the page. Durable history in the browser means OPFS, which needs
//! a VFS installed from a dedicated worker; that was built and then backed
//! out (LOG.md section 16).

use calc_core::{Calculator, MemoryHistory};
use calc_history_sqlite::SqliteHistory;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum KeyStyle {
    Input,
    Function,
    Operator,
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone)]
pub struct KeySpec {
    pub id: String,
    pub label: String,
    pub style: KeyStyle,
    pub span: u32,
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone)]
pub struct KeyRow {
    pub keys: Vec<KeySpec>,
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone)]
pub struct HistoryLine {
    pub id: u64,
    pub expression: String,
    pub result: String,
}

#[wasm_bindgen(getter_with_clone)]
pub struct ViewModel {
    pub expression: String,
    pub display: String,
    #[wasm_bindgen(js_name = isError)]
    pub is_error: bool,
    pub history: Vec<HistoryLine>,
    pub caption: String,
}

impl From<calc_ui::KeyStyle> for KeyStyle {
    fn from(style: calc_ui::KeyStyle) -> Self {
        match style {
            calc_ui::KeyStyle::Input => KeyStyle::Input,
            calc_ui::KeyStyle::Function => KeyStyle::Function,
            calc_ui::KeyStyle::Operator => KeyStyle::Operator,
        }
    }
}

impl From<calc_ui::KeyRow> for KeyRow {
    fn from(row: calc_ui::KeyRow) -> Self {
        KeyRow {
            keys: row
                .keys
                .into_iter()
                .map(|k| KeySpec { id: k.id, label: k.label, style: k.style.into(), span: k.span })
                .collect(),
        }
    }
}

impl From<calc_ui::ViewModel> for ViewModel {
    fn from(vm: calc_ui::ViewModel) -> Self {
        ViewModel {
            expression: vm.expression,
            display: vm.display,
            is_error: vm.is_error,
            history: vm
                .history
                .into_iter()
                .map(|h| HistoryLine { id: h.id, expression: h.expression, result: h.result })
                .collect(),
            caption: vm.caption,
        }
    }
}

/// The keypad. Static, so the renderer asks for it once when it builds the
/// screen rather than on every keystroke.
#[wasm_bindgen]
pub fn keypad() -> Vec<KeyRow> {
    calc_ui::keypad().into_iter().map(Into::into).collect()
}

/// The calculator screen.
#[wasm_bindgen]
pub struct CalculatorUi {
    inner: calc_ui::CalculatorUi,
}

#[wasm_bindgen]
impl CalculatorUi {
    /// A calculator whose history is a SQLite database in memory.
    ///
    /// Cannot fail: if SQLite will not open even that, history falls back to
    /// a plain `Vec` carrying the reason, and `view().caption` says so.
    #[wasm_bindgen(constructor)]
    pub fn new() -> CalculatorUi {
        let calculator = SqliteHistory::in_memory()
            .and_then(|store| Calculator::with_store(Box::new(store)))
            .unwrap_or_else(|e| {
                Calculator::with_store(Box::new(MemoryHistory::fallback(e.message)))
                    .expect("the memory store always reads back")
            });
        CalculatorUi { inner: calc_ui::CalculatorUi::with_calculator(calculator) }
    }

    /// The current state, for the initial render.
    pub fn view(&self) -> ViewModel {
        self.inner.view().into()
    }

    /// An on-screen key was tapped.
    pub fn press(&mut self, id: &str) -> ViewModel {
        self.inner.press(id).into()
    }

    /// A `KeyboardEvent.key`: "Enter", "Backspace", "Escape" or one
    /// character. `undefined` means the calculator ignores it.
    #[wasm_bindgen(js_name = keyInput)]
    pub fn key_input(&mut self, key: &str) -> Option<ViewModel> {
        self.inner.key_input(key).map(Into::into)
    }
}

impl Default for CalculatorUi {
    fn default() -> Self {
        Self::new()
    }
}
