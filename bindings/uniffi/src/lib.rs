//! UniFFI surface over `calc_ui` for Swift (Apple) and Kotlin (Android).
//!
//! Proc-macro mode: no .udl file and no build.rs. Bindings are generated
//! from the compiled library's embedded metadata (see //bindings/uniffi:BUILD).
//!
//! The records a renderer draws are `calc_ui`'s own types, declared below as
//! UniFFI *remote* types: each declaration repeats the type's shape, and
//! UniFFI implements its FFI traits for the real `calc_ui` type from that.
//! The repeat is checked by the compiler. Lifting builds the real struct
//! from exactly the fields listed here, so a field added to or removed from
//! `calc_ui` without a matching edit below is a build error, and the same
//! goes for enum variants through an exhaustive `match`. There is no mirror
//! type and no conversion: `CalculatorUi::view` returns `calc_ui`'s
//! `ViewModel` as it is.
//!
//! Why not put the derives on `calc_ui` behind a crate feature, the way the
//! wasm binding does? UniFFI would then treat `calc_ui` as a second
//! component and generate a second Swift file, a second Kotlin package and a
//! second set of scaffolding, and the bindgen would have to read the cdylib
//! instead of the rlib (LOG.md section 17). The remote declarations cost a
//! few checked lines here and leave all of that alone.

use std::sync::Mutex;

use calc_ui::{HistoryLine, KeyRow, KeySpec, KeyStyle, ViewModel};

uniffi::setup_scaffolding!();

#[uniffi::remote(Enum)]
pub enum KeyStyle {
    Input,
    Function,
    Operator,
}

#[uniffi::remote(Record)]
pub struct KeySpec {
    pub id: String,
    pub label: String,
    pub style: KeyStyle,
    pub span: u32,
}

#[uniffi::remote(Record)]
pub struct KeyRow {
    pub keys: Vec<KeySpec>,
}

#[uniffi::remote(Record)]
pub struct HistoryLine {
    pub id: u64,
    pub expression: String,
    pub result: String,
}

#[uniffi::remote(Record)]
pub struct ViewModel {
    pub expression: String,
    pub display: String,
    pub is_error: bool,
    pub history: Vec<HistoryLine>,
    pub caption: String,
}

/// The keypad. Static, so a renderer asks for it once when it builds the
/// screen rather than on every keystroke.
#[uniffi::export]
pub fn keypad() -> Vec<KeyRow> {
    calc_ui::keypad()
}

/// Width of the keypad grid in columns; every row's spans add up to it. A
/// renderer sizes its grid from this rather than repeating the number.
#[uniffi::export]
pub fn keypad_columns() -> u32 {
    calc_ui::KEYPAD_COLUMNS
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
