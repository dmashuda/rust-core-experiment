//! UniFFI surface over `calc_ui` for Swift (Apple) and Kotlin (Android).
//!
//! Proc-macro mode: no .udl file and no build.rs. Bindings are generated
//! from the compiled library's embedded metadata (see //bindings/uniffi:BUILD).
//!
//! The records a renderer draws (`ViewModel`, `KeyRow`, ...) are `calc_ui`'s
//! own types: this crate links `//core/calc_ui:calc_ui_uniffi`, which turns
//! on the `uniffi` feature that puts the UniFFI derives on them. The bindgen
//! then sees two components in one library, `calc_ui` for the records and
//! `calc_ffi` for the object below, and emits a Swift file and a Kotlin file
//! for each.

use std::sync::Mutex;

use calc_ui::{KeyRow, ViewModel};

uniffi::setup_scaffolding!();

/// The keypad. Static, so a renderer asks for it once when it builds the
/// screen rather than on every keystroke.
#[uniffi::export]
pub fn keypad() -> Vec<KeyRow> {
    calc_ui::keypad()
}

/// The calculator screen. UniFFI objects are shared (`Arc`) across the FFI
/// boundary and may be called from any thread, hence the `Mutex`.
#[derive(uniffi::Object)]
pub struct CalculatorUi {
    inner: Mutex<calc_ui::CalculatorUi>,
}

#[uniffi::export]
impl CalculatorUi {
    /// A calculator whose history is not kept past this object.
    #[uniffi::constructor]
    pub fn new() -> Self {
        CalculatorUi { inner: Mutex::new(calc_ui::CalculatorUi::new()) }
    }

    /// A calculator whose history is the SQLite database at `db_path`,
    /// created if it is not there yet. Only the platform knows where it may
    /// write (an app sandbox), so only the platform passes the path.
    ///
    /// This cannot fail: an unusable path leaves the calculator working on an
    /// in-memory history, and `view().caption` says so.
    #[uniffi::constructor]
    pub fn open(db_path: String) -> Self {
        let calculator = calc_history_sqlite::open_calculator(&db_path);
        CalculatorUi { inner: Mutex::new(calc_ui::CalculatorUi::with_calculator(calculator)) }
    }

    /// The current state, for the initial render.
    pub fn view(&self) -> ViewModel {
        self.inner.lock().unwrap().view()
    }

    /// An on-screen key was tapped.
    pub fn press(&self, id: String) -> ViewModel {
        self.inner.lock().unwrap().press(&id)
    }

    /// A hardware key: "Enter", "Backspace", "Escape" or one character.
    /// `None` means the calculator ignores it.
    pub fn key_input(&self, key: String) -> Option<ViewModel> {
        self.inner.lock().unwrap().key_input(&key)
    }
}

impl Default for CalculatorUi {
    fn default() -> Self {
        Self::new()
    }
}
