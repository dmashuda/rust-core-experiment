//! wasm-bindgen surface over `calc_ui` for the web front end.
//!
//! The classes JS sees for the view model (`ViewModel`, `KeyRow`, ...) are
//! `calc_ui`'s own types: this crate links `//core/calc_ui:calc_ui_wasm`,
//! which turns on the `wasm` feature that puts the wasm-bindgen attributes on
//! them (getters via `getter_with_clone`). wasm-bindgen frees them through a
//! FinalizationRegistry once JS drops them, so the renderer never has to
//! call `.free()`.
//!
//! History is SQLite here too -- the same crate and schema the native apps
//! use, compiled to wasm -- but held in SQLite's own memory, so it lasts as
//! long as the page. Durable history in the browser means OPFS, which needs
//! a VFS installed from a dedicated worker; that was built and then backed
//! out (LOG.md section 16).

use calc_core::{Calculator, MemoryHistory};
use calc_history_sqlite::SqliteHistory;
use calc_ui::{KeyRow, ViewModel};
use wasm_bindgen::prelude::*;

/// The keypad. Static, so the renderer asks for it once when it builds the
/// screen rather than on every keystroke.
#[wasm_bindgen]
pub fn keypad() -> Vec<KeyRow> {
    calc_ui::keypad()
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
        self.inner.view()
    }

    /// An on-screen key was tapped.
    pub fn press(&mut self, id: &str) -> ViewModel {
        self.inner.press(id)
    }

    /// A `KeyboardEvent.key`: "Enter", "Backspace", "Escape" or one
    /// character. `undefined` means the calculator ignores it.
    #[wasm_bindgen(js_name = keyInput)]
    pub fn key_input(&mut self, key: &str) -> Option<ViewModel> {
        self.inner.key_input(key)
    }
}

impl Default for CalculatorUi {
    fn default() -> Self {
        Self::new()
    }
}
