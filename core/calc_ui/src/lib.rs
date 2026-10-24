//! Calculator UI state, shared by every front end.
//!
//! Front ends are thin renderers: they draw a [`ViewModel`] and report user
//! input back through [`CalculatorUi::press`] (on-screen keys) or
//! [`CalculatorUi::key_input`] (hardware keyboard). Keypad layout, input
//! editing, evaluation, error state and history all live here.
//!
//! The keypad is static, so it comes from [`keypad`] once when a screen is
//! built. [`ViewModel`] carries only what changes between keystrokes.

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
    /// Stable for the life of the entry: a renderer keys its row views on
    /// this, so identity survives newer lines being prepended.
    pub id: u64,
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

/// The keypad, row by row. Static: build the keys once when a screen is
/// created rather than on every keystroke.
pub fn keypad() -> Vec<KeyRow> {
    KEYPAD
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
        .collect()
}

/// Operators that continue from the previous answer when they are the first
/// thing pressed after `=`, the way a desk calculator does.
const CONTINUATION_OPERATORS: &[&str] = &["+", "-", "*", "/", "%", "^"];

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
    /// What `display` held before an error replaced it, so editing the
    /// expression can put it back instead of leaving stale error text on
    /// screen styled as a normal result.
    display_before_error: Option<String>,
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
            display_before_error: None,
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
                .map(|e| HistoryLine {
                    id: e.id,
                    expression: e.expression.clone(),
                    result: e.result.clone(),
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
                        self.insert_text(c.encode_utf8(&mut [0u8; 4]));
                        return Some(self.view());
                    }
                    _ => return None,
                }
            }
        };
        self.apply(action);
        Some(self.view())
    }

    /// Appends `text` to the expression. An operator typed straight after `=`
    /// continues from the answer, so `6*7=` then `+1=` gives 43.
    fn insert_text(&mut self, text: &str) {
        if self.expression.is_empty()
            && CONTINUATION_OPERATORS.contains(&text)
            && self.calculator.last_result().is_some()
        {
            self.expression.push_str("ans");
        }
        self.expression.push_str(text);
        self.leave_error();
    }

    /// Leaves error state, restoring the display the error message replaced.
    fn leave_error(&mut self) {
        if self.is_error {
            self.is_error = false;
            self.display = self.display_before_error.take().unwrap_or_else(|| "0".to_string());
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Insert(text) => self.insert_text(text),
            Action::Clear => {
                // Reset the whole screen like a calculator's C key. History
                // and `ans` are kept.
                self.expression.clear();
                self.display = "0".to_string();
                self.is_error = false;
                self.display_before_error = None;
            }
            Action::Backspace => {
                self.expression.pop();
                self.leave_error();
            }
            Action::Evaluate => match self.calculator.evaluate(&self.expression) {
                Ok(result) => {
                    self.display = result;
                    self.is_error = false;
                    self.display_before_error = None;
                    self.expression.clear();
                }
                Err(e) => {
                    if !self.is_error {
                        self.display_before_error = Some(self.display.clone());
                    }
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
                HistoryLine { id: 2, expression: "ans/2".into(), result: "21".into() },
                HistoryLine { id: 1, expression: "6*7".into(), result: "42".into() },
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
    fn clear_resets_the_display_after_a_result_or_error() {
        let mut ui = CalculatorUi::new();
        press_all(&mut ui, &["6", "*", "7", "evaluate"]);
        let vm = ui.press("clear");
        assert_eq!(vm.display, "0");
        assert_eq!(vm.history.len(), 1, "history survives clear");

        press_all(&mut ui, &["1", "/", "0", "evaluate"]);
        let vm = ui.key_input("Escape").unwrap();
        assert_eq!((vm.display.as_str(), vm.is_error, vm.expression.as_str()), ("0", false, ""));

        assert_eq!(press_all(&mut ui, &["ans", "evaluate"]).display, "42", "ans survives clear");
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
        assert!(ui.key_input("=").unwrap().is_error, "empty expression is an error");
    }
    #[test]
    fn editing_after_an_error_restores_the_display() {
        let mut ui = CalculatorUi::new();
        press_all(&mut ui, &["1", "/", "0", "evaluate"]);
        let vm = ui.press("backspace");
        assert_eq!(vm.display, "0", "no earlier result, so the display goes back to 0");
        assert!(!vm.is_error);

        let mut ui = CalculatorUi::new();
        press_all(&mut ui, &["6", "*", "7", "evaluate"]);
        press_all(&mut ui, &["1", "/", "0", "evaluate"]);
        assert_eq!(ui.view().display, "division by zero");
        let vm = ui.press("5");
        assert_eq!(vm.display, "42", "the last good result comes back; the error text is gone");
        assert!(!vm.is_error);
    }

    #[test]
    fn operators_continue_from_the_last_result() {
        let mut ui = CalculatorUi::new();
        press_all(&mut ui, &["6", "*", "7", "evaluate"]);
        let vm = ui.press("+");
        assert_eq!(vm.expression, "ans+", "an operator after = continues from the answer");
        assert_eq!(press_all(&mut ui, &["1", "evaluate"]).display, "43");
    }

    #[test]
    fn typed_operators_continue_from_the_last_result() {
        let mut ui = CalculatorUi::new();
        for c in "6*7".chars() {
            ui.key_input(&c.to_string());
        }
        ui.key_input("Enter");
        assert_eq!(ui.key_input("*").unwrap().expression, "ans*");
    }

    #[test]
    fn operators_do_not_seed_ans_without_a_previous_answer() {
        let mut ui = CalculatorUi::new();
        assert_eq!(ui.press("+").expression, "+");
        let mut ui = CalculatorUi::new();
        assert_eq!(press_all(&mut ui, &["1", "+"]).expression, "1+", "mid-expression is untouched");
    }

    #[test]
    fn digits_after_a_result_start_a_new_expression() {
        let mut ui = CalculatorUi::new();
        press_all(&mut ui, &["6", "*", "7", "evaluate"]);
        assert_eq!(press_all(&mut ui, &["5"]).expression, "5");
    }

    #[test]
    fn the_keypad_is_static_and_not_part_of_the_view_model() {
        let rows = keypad();
        let mut ids = std::collections::HashSet::new();
        for row in &rows {
            assert_eq!(row.keys.iter().map(|k| k.span).sum::<u32>(), KEYPAD_COLUMNS);
            for k in &row.keys {
                assert!(k.span >= 1);
                assert!(ids.insert(k.id.clone()), "duplicate key id {}", k.id);
            }
        }
        assert!(ids.contains("evaluate"));
        assert_eq!(keypad(), rows, "the keypad is the same every time: fetch it once");
    }

    #[test]
    fn the_view_models_history_is_capped() {
        let mut ui = CalculatorUi::new();
        for _ in 0..calc_core::HISTORY_LIMIT + 10 {
            press_all(&mut ui, &["1", "evaluate"]);
        }
        assert_eq!(ui.view().history.len(), calc_core::HISTORY_LIMIT);
    }

    #[test]
    fn history_lines_carry_stable_ids() {
        let mut ui = CalculatorUi::new();
        press_all(&mut ui, &["1", "evaluate"]);
        let first = ui.view().history[0].id;
        press_all(&mut ui, &["2", "evaluate"]);
        let vm = ui.view();
        assert_eq!(vm.history[1].id, first, "an older line keeps its id when a newer one is prepended");
        assert_ne!(vm.history[0].id, first);
    }
}
