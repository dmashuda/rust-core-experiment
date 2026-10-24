//! Calculator UI state, shared by every front end.
//!
//! Front ends are thin renderers: they draw a [`ViewModel`] and report user
//! input back through [`CalculatorUi::press`] (on-screen keys) or
//! [`CalculatorUi::key_input`] (hardware keyboard). Keypad layout, input
//! editing, evaluation, error state and history all live here.

use calc_core::Calculator;

/// Visual role of a key; renderers map it to colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStyle {
    /// Digits, the decimal point and similar inputs.
    Input,
    /// Parentheses, `^`, `%`, `ans`, clear and backspace.
    Function,
    /// The arithmetic operators and `=` (highlighted).
    Operator,
}

/// One on-screen key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySpec {
    /// Stable identifier passed back to [`CalculatorUi::press`].
    pub id: String,
    /// Text to show on the key.
    pub label: String,
    pub style: KeyStyle,
    /// Number of grid columns the key spans (at least 1).
    pub span: u32,
}

/// One keypad row. Each row's spans add up to [`KEYPAD_COLUMNS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRow {
    pub keys: Vec<KeySpec>,
}

/// One history line, newest first in [`ViewModel::history`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryLine {
    pub expression: String,
    pub result: String,
}

/// Everything a renderer needs to draw the calculator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewModel {
    /// The expression being typed (may be empty).
    pub expression: String,
    /// The main display: the last result, or an error message.
    pub display: String,
    /// True when `display` holds an error message.
    pub is_error: bool,
    pub history: Vec<HistoryLine>,
    pub keypad: Vec<KeyRow>,
    /// Footer caption.
    pub caption: String,
}

/// Width of the keypad grid in columns.
pub const KEYPAD_COLUMNS: u32 = 4;

#[derive(Clone, Copy)]
enum Action {
    Insert(&'static str),
    Clear,
    Backspace,
    Evaluate,
}

struct Key {
    id: &'static str,
    label: &'static str,
    style: KeyStyle,
    span: u32,
    action: Action,
}

const fn key(id: &'static str, label: &'static str, style: KeyStyle, action: Action) -> Key {
    Key { id, label, style, span: 1, action }
}

const fn input(text: &'static str) -> Key {
    key(text, text, KeyStyle::Input, Action::Insert(text))
}

const fn func(text: &'static str) -> Key {
    key(text, text, KeyStyle::Function, Action::Insert(text))
}

const fn op(id: &'static str, label: &'static str) -> Key {
    key(id, label, KeyStyle::Operator, Action::Insert(id))
}

/// The keypad, row by row. Changing this changes every app.
const KEYPAD: &[&[Key]] = &[
    &[key("clear", "C", KeyStyle::Function, Action::Clear), func("("), func(")"), op("/", "÷")],
    &[input("7"), input("8"), input("9"), op("*", "×")],
    &[input("4"), input("5"), input("6"), op("-", "−")],
    &[input("1"), input("2"), input("3"), op("+", "+")],
    &[input("0"), input("."), func("^"), key("backspace", "⌫", KeyStyle::Function, Action::Backspace)],
    &[
        func("ans"),
        func("%"),
        Key { id: "evaluate", label: "=", style: KeyStyle::Operator, span: 2, action: Action::Evaluate },
    ],
];

/// Characters a hardware keyboard may type into the expression. Letters are
/// allowed so `ans` can be typed.
fn is_typeable(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_lowercase() || ".+-*/%^() ".contains(c)
}

/// The calculator screen's state machine.
pub struct CalculatorUi {
    calculator: Calculator,
    expression: String,
    display: String,
    is_error: bool,
}

impl Default for CalculatorUi {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorUi {
    pub fn new() -> Self {
        CalculatorUi {
            calculator: Calculator::new(),
            expression: String::new(),
            display: "0".to_string(),
            is_error: false,
        }
    }

    /// The current state, for the initial render.
    pub fn view(&self) -> ViewModel {
        ViewModel {
            expression: self.expression.clone(),
            display: self.display.clone(),
            is_error: self.is_error,
            history: self
                .calculator
                .history()
                .iter()
                .rev()
                .map(|e| HistoryLine { expression: e.expression.clone(), result: e.result.clone() })
                .collect(),
            keypad: KEYPAD
                .iter()
                .map(|row| KeyRow {
                    keys: row
                        .iter()
                        .map(|k| KeySpec {
                            id: k.id.to_string(),
                            label: k.label.to_string(),
                            style: k.style,
                            span: k.span,
                        })
                        .collect(),
                })
                .collect(),
            caption: "Evaluated by Rust".to_string(),
        }
    }

    /// Handles a tap on the on-screen key with this `id`. Unknown ids are
    /// ignored.
    pub fn press(&mut self, id: &str) -> ViewModel {
        if let Some(k) = KEYPAD.iter().flat_map(|row| row.iter()).find(|k| k.id == id) {
            self.apply(k.action);
        }
        self.view()
    }

    /// Handles a hardware key. `key` is `"Enter"`, `"Backspace"`, `"Escape"`,
    /// or a single typed character. Returns `None` if the key is not one the
    /// calculator uses, so the platform can let it propagate.
    pub fn key_input(&mut self, key: &str) -> Option<ViewModel> {
        let action = match key {
            "Enter" | "=" => Action::Evaluate,
            "Backspace" => Action::Backspace,
            "Escape" => Action::Clear,
            _ => {
                let mut chars = key.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if is_typeable(c) => {
                        self.expression.push(c);
                        self.is_error = false;
                        return Some(self.view());
                    }
                    _ => return None,
                }
            }
        };
        self.apply(action);
        Some(self.view())
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Insert(text) => {
                self.expression.push_str(text);
                self.is_error = false;
            }
            Action::Clear => {
                self.expression.clear();
                self.is_error = false;
            }
            Action::Backspace => {
                self.expression.pop();
                self.is_error = false;
            }
            Action::Evaluate => match self.calculator.evaluate(&self.expression) {
                Ok(result) => {
                    self.display = result;
                    self.is_error = false;
                    self.expression.clear();
                }
                Err(e) => {
                    self.display = e.to_string();
                    self.is_error = true;
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press_all(ui: &mut CalculatorUi, ids: &[&str]) -> ViewModel {
        let mut vm = ui.view();
        for id in ids {
            vm = ui.press(id);
        }
        vm
    }

    #[test]
    fn initial_state() {
        let vm = CalculatorUi::new().view();
        assert_eq!(vm.expression, "");
        assert_eq!(vm.display, "0");
        assert!(!vm.is_error);
        assert!(vm.history.is_empty());
    }

    #[test]
    fn keypad_rows_fill_the_grid_and_ids_are_unique() {
        let vm = CalculatorUi::new().view();
        let mut ids = std::collections::HashSet::new();
        for row in &vm.keypad {
            assert_eq!(row.keys.iter().map(|k| k.span).sum::<u32>(), KEYPAD_COLUMNS);
            for k in &row.keys {
                assert!(k.span >= 1);
                assert!(ids.insert(k.id.clone()), "duplicate key id {}", k.id);
            }
        }
    }

    #[test]
    fn keys_build_and_evaluate_an_expression() {
        let mut ui = CalculatorUi::new();
        let vm = press_all(&mut ui, &["2", "*", "(", "3", "+", "4", ")", "^", "2"]);
        assert_eq!(vm.expression, "2*(3+4)^2");
        let vm = ui.press("evaluate");
        assert_eq!(vm.display, "98");
        assert_eq!(vm.expression, "");
        assert!(!vm.is_error);
    }

    #[test]
    fn history_is_newest_first_and_ans_works() {
        let mut ui = CalculatorUi::new();
        press_all(&mut ui, &["6", "*", "7", "evaluate"]);
        let vm = press_all(&mut ui, &["ans", "/", "2", "evaluate"]);
        assert_eq!(vm.display, "21");
        assert_eq!(
            vm.history,
            vec![
                HistoryLine { expression: "ans/2".into(), result: "21".into() },
                HistoryLine { expression: "6*7".into(), result: "42".into() },
            ]
        );
    }

    #[test]
    fn errors_keep_the_expression_and_clear_on_edit() {
        let mut ui = CalculatorUi::new();
        let vm = press_all(&mut ui, &["1", "/", "0", "evaluate"]);
        assert!(vm.is_error);
        assert_eq!(vm.display, "division by zero");
        assert_eq!(vm.expression, "1/0", "expression is kept so it can be fixed");
        assert!(vm.history.is_empty());

        let vm = ui.press("backspace");
        assert_eq!(vm.expression, "1/");
        assert!(!vm.is_error);
    }

    #[test]
    fn clear_and_backspace() {
        let mut ui = CalculatorUi::new();
        let vm = press_all(&mut ui, &["1", "2", "backspace"]);
        assert_eq!(vm.expression, "1");
        let vm = ui.press("clear");
        assert_eq!(vm.expression, "");
        let vm = ui.press("backspace");
        assert_eq!(vm.expression, "", "backspace on empty is a no-op");
    }

    #[test]
    fn unknown_key_id_is_ignored() {
        let mut ui = CalculatorUi::new();
        assert_eq!(ui.press("nope"), CalculatorUi::new().view());
    }

    #[test]
    fn keyboard_input() {
        let mut ui = CalculatorUi::new();
        for c in "ans".chars().chain("1+2".chars()) {
            assert!(ui.key_input(&c.to_string()).is_some());
        }
        ui.key_input("Escape");
        for c in "1+2".chars() {
            ui.key_input(&c.to_string());
        }
        assert_eq!(ui.key_input("Enter").unwrap().display, "3");
        assert_eq!(ui.key_input("4").unwrap().expression, "4");
        assert_eq!(ui.key_input("Backspace").unwrap().expression, "");
        assert!(ui.key_input("Tab").is_none(), "unknown named keys propagate");
        assert!(ui.key_input("A").is_none(), "uppercase is not typeable");
        assert!(ui.key_input("").is_none());
        assert_eq!(ui.key_input("=").unwrap().is_error, true, "empty expression is an error");
    }
}
