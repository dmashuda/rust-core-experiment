//! UniFFI surface over `calc_ui` for Swift (Apple) and Kotlin (Android).
//!
//! Proc-macro mode: no .udl file and no build.rs. Bindings are generated
//! from the compiled library's embedded metadata (see //bindings/uniffi:BUILD).
//!
//! The records below mirror `calc_ui`'s types: UniFFI derives have to live in
//! the crate that defines the type, and the core crates stay FFI-free.

use std::sync::Mutex;

uniffi::setup_scaffolding!();

#[derive(uniffi::Enum)]
pub enum KeyStyle {
    Input,
    Function,
    Operator,
}

#[derive(uniffi::Record)]
pub struct KeySpec {
    pub id: String,
    pub label: String,
    pub style: KeyStyle,
    pub span: u32,
}

#[derive(uniffi::Record)]
pub struct KeyRow {
    pub keys: Vec<KeySpec>,
}

#[derive(uniffi::Record)]
pub struct HistoryLine {
    pub expression: String,
    pub result: String,
}

#[derive(uniffi::Record)]
pub struct ViewModel {
    pub expression: String,
    pub display: String,
    pub is_error: bool,
    pub history: Vec<HistoryLine>,
    pub keypad: Vec<KeyRow>,
    pub caption: String,
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
                .map(|h| HistoryLine { expression: h.expression, result: h.result })
                .collect(),
            keypad: vm
                .keypad
                .into_iter()
                .map(|row| KeyRow {
                    keys: row
                        .keys
                        .into_iter()
                        .map(|k| KeySpec {
                            id: k.id,
                            label: k.label,
                            style: match k.style {
                                calc_ui::KeyStyle::Input => KeyStyle::Input,
                                calc_ui::KeyStyle::Function => KeyStyle::Function,
                                calc_ui::KeyStyle::Operator => KeyStyle::Operator,
                            },
                            span: k.span,
                        })
                        .collect(),
                })
                .collect(),
            caption: vm.caption,
        }
    }
}

/// The calculator screen. UniFFI objects are shared (`Arc`) across the FFI
/// boundary and may be called from any thread, hence the `Mutex`.
#[derive(uniffi::Object)]
pub struct CalculatorUi {
    inner: Mutex<calc_ui::CalculatorUi>,
}

#[uniffi::export]
impl CalculatorUi {
    #[uniffi::constructor]
    pub fn new() -> Self {
        CalculatorUi { inner: Mutex::new(calc_ui::CalculatorUi::new()) }
    }

    /// The current state, for the initial render.
    pub fn view(&self) -> ViewModel {
        self.inner.lock().unwrap().view().into()
    }

    /// An on-screen key was tapped.
    pub fn press(&self, id: String) -> ViewModel {
        self.inner.lock().unwrap().press(&id).into()
    }

    /// A hardware key: "Enter", "Backspace", "Escape" or one character.
    /// `None` means the calculator ignores it.
    pub fn key_input(&self, key: String) -> Option<ViewModel> {
        self.inner.lock().unwrap().key_input(&key).map(Into::into)
    }
}
