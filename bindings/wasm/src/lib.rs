//! wasm-bindgen surface over `calc_ui` for the web front end.
//!
//! Structs are exported as JS classes with getters (`getter_with_clone`).
//! wasm-bindgen frees them through a FinalizationRegistry once JS drops
//! them, so the renderer never has to call `.free()`.
//!
//! [`open_calculator`] keeps history in SQLite, in OPFS. That is why the web
//! app runs this module in a dedicated worker: the VFS it installs needs
//! `createSyncAccessHandle`, which only a worker has.

use calc_history_sqlite::SqliteHistory;
use rusqlite::Connection;
use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfg, install};
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

/// The hardware keys the calculator acts on.
///
/// The renderer needs this because the calculator runs in a worker: whether
/// to call `preventDefault()` has to be decided while the key event is still
/// being dispatched, which is too early to have asked the worker. Static, so
/// it is fetched once with the keypad.
#[wasm_bindgen(getter_with_clone)]
pub struct KeyboardKeys {
    pub named: Vec<String>,
    pub typeable: String,
}

#[wasm_bindgen(js_name = keyboardKeys)]
pub fn keyboard_keys() -> KeyboardKeys {
    let keys = calc_ui::keyboard_keys();
    KeyboardKeys { named: keys.named, typeable: keys.typeable }
}

/// The calculator screen.
#[wasm_bindgen]
pub struct CalculatorUi {
    inner: calc_ui::CalculatorUi,
}

#[wasm_bindgen]
impl CalculatorUi {
    #[wasm_bindgen(constructor)]
    pub fn new() -> CalculatorUi {
        CalculatorUi { inner: calc_ui::CalculatorUi::new() }
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

/// Opens a calculator whose history is the SQLite database `db_name`, kept in
/// the origin's private file system (OPFS) so it survives a reload.
///
/// Async because acquiring the OPFS sync access handles is, and fallible only
/// in the sense that it degrades: outside a worker, outside a secure context,
/// or in a browser without OPFS, the database is held in memory instead and
/// `view().caption` says which happened. The calculator always works.
#[wasm_bindgen(js_name = openCalculator)]
pub async fn open_calculator(db_name: String) -> CalculatorUi {
    let store = match install::<sqlite_wasm_rs::WasmOsCallback>(&OpfsSAHPoolCfg::default(), true).await {
        // The pool is now SQLite's default VFS, so a plain open lands in it.
        Ok(_) => Connection::open(&db_name)
            .map_err(|e| e.to_string())
            .and_then(|conn| SqliteHistory::from_connection(conn, "OPFS").map_err(|e| e.message)),
        Err(e) => Err(e.to_string()),
    };
    let store = match store {
        Ok(store) => Ok(store),
        // Still SQLite, still the same schema and queries -- just nowhere to
        // put the bytes.
        Err(reason) => SqliteHistory::from_connection(
            Connection::open_in_memory().expect("SQLite always opens an in-memory database"),
            &format!("memory, no OPFS: {reason}"),
        )
        .map_err(|e| e.message),
    };
    let calculator = match store {
        Ok(store) => calc_core::Calculator::with_store(Box::new(store))
            .unwrap_or_else(|e| fallback_calculator(e.message)),
        Err(reason) => fallback_calculator(reason),
    };
    CalculatorUi { inner: calc_ui::CalculatorUi::with_calculator(calculator) }
}

/// Last resort: a calculator with no store at all, carrying the reason into
/// the footer so the page does not silently stop keeping history.
fn fallback_calculator(reason: String) -> calc_core::Calculator {
    calc_core::Calculator::with_store(Box::new(calc_core::MemoryHistory::fallback(reason)))
        .expect("the memory store always reads back")
}

/// Tests that need a browser: OPFS exists nowhere else.
///
/// The Node test (`//bindings/wasm:calc_wasm_test`) covers the API and the
/// in-memory fallback, which is all Node can reach. This one proves the
/// history is actually durable, and runs in headless Chrome through
/// `//bindings/wasm:calc_wasm_browser_test`.
#[cfg(test)]
mod browser_tests {
    use super::*;
    use wasm_bindgen_test::*;

    // A dedicated worker, because `createSyncAccessHandle` is only there --
    // the same reason the web app runs the calculator in one.
    wasm_bindgen_test_configure!(run_in_dedicated_worker);

    #[wasm_bindgen_test]
    async fn history_is_kept_in_opfs_across_calculators() {
        let database = "browser-test.db".to_string();
        let mut ui = open_calculator(database.clone()).await;

        let caption = ui.view().caption;
        assert!(caption.ends_with("(OPFS)"), "history should be in OPFS, not: {caption}");

        // Counted rather than assumed empty: OPFS outlives a page, so a
        // previous run of this test may have left rows behind.
        let before = ui.view().history.len();
        for id in ["6", "*", "7", "evaluate"] {
            ui.press(id);
        }
        let view = ui.view();
        assert_eq!(view.display, "42");
        assert_eq!(view.history.len(), before + 1);

        // Close it and open the database again: a reload, in effect.
        drop(ui);
        let reopened = open_calculator(database).await;
        let view = reopened.view();
        assert_eq!(view.history.len(), before + 1, "the row was not read back out of OPFS");
        let newest = &view.history[0];
        assert_eq!((newest.expression.as_str(), newest.result.as_str()), ("6*7", "42"));
        assert_eq!(view.display, "42", "and `ans` came back with it");
    }
}
