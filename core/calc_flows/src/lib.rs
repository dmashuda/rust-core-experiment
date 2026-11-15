//! Test flows shared by every front end.
//!
//! A [`Flow`] is a list of [`Step`]s: what the user does (tap an on-screen
//! key, type on a hardware keyboard) and what the view model should then
//! say. Keys are named by the ids `//core/calc_ui`'s keypad gives them.
//!
//! [`run`] plays a flow against the Rust view model. Each front end has a
//! driver that plays the same flows, read from `flows.json`, through its own
//! bindings, model and (on the web) widgets. A driver is a page of code and
//! knows nothing about arithmetic. Besides the steps, it must start each
//! flow on a new calculator with an empty history.

pub mod flows;

use calc_ui::{CalculatorUi, ViewModel};

/// One scenario, starting from a new calculator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Tap the on-screen key with this id. It must be on the keypad.
    Press { id: String },
    /// A hardware key the calculator handles: `"Enter"`, `"Backspace"`,
    /// `"Escape"` or one character (what `key_input` takes).
    Key { key: String },
    /// A hardware key the calculator must leave to the platform.
    Ignored { key: String },
    /// The view model's `field` reads `value`.
    Expect { field: Field, value: String },
}

/// What [`Step::Expect`] can read from the view model, as a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Expression,
    Display,
    /// `true` or `false`: whether the display holds an error.
    Error,
    /// One `expression = result` line per entry, newest first, joined by
    /// newlines; empty when there is no history.
    History,
}

impl Field {
    /// How `flows.json` spells it.
    pub fn name(self) -> &'static str {
        match self {
            Field::Expression => "expression",
            Field::Display => "display",
            Field::Error => "error",
            Field::History => "history",
        }
    }
}

impl Flow {
    pub fn new(name: &str) -> Flow {
        Flow { name: name.to_string(), steps: Vec::new() }
    }

    fn step(mut self, step: Step) -> Flow {
        self.steps.push(step);
        self
    }

    /// Taps each key in turn.
    pub fn press(self, ids: &[&str]) -> Flow {
        ids.iter().fold(self, |flow, id| flow.step(Step::Press { id: id.to_string() }))
    }

    pub fn key(self, key: &str) -> Flow {
        self.step(Step::Key { key: key.to_string() })
    }

    /// Types each character as its own key.
    pub fn typed(self, text: &str) -> Flow {
        text.chars().fold(self, |flow, c| flow.key(&c.to_string()))
    }

    pub fn ignored(self, key: &str) -> Flow {
        self.step(Step::Ignored { key: key.to_string() })
    }

    pub fn expect(self, field: Field, value: &str) -> Flow {
        self.step(Step::Expect { field, value: value.to_string() })
    }

    pub fn expression(self, value: &str) -> Flow {
        self.expect(Field::Expression, value)
    }

    pub fn display(self, value: &str) -> Flow {
        self.expect(Field::Display, value)
    }

    pub fn error(self, value: bool) -> Flow {
        self.expect(Field::Error, &value.to_string())
    }

    /// The whole history, newest first.
    pub fn history(self, lines: &[&str]) -> Flow {
        self.expect(Field::History, &lines.join("\n"))
    }
}

/// `field` of `vm` as a string.
pub fn read(vm: &ViewModel, field: Field) -> String {
    match field {
        Field::Expression => vm.expression.clone(),
        Field::Display => vm.display.clone(),
        Field::Error => vm.is_error.to_string(),
        Field::History => vm
            .history
            .iter()
            .map(|l| format!("{} = {}", l.expression, l.result))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Plays `flow` against the Rust view model, over a real SQLite history at
/// `db_path` (which must not exist yet). The error says which step failed.
pub fn run(flow: &Flow, db_path: &str) -> Result<(), String> {
    let mut ui = CalculatorUi::with_calculator(calc_history_sqlite::open_calculator(db_path));
    let keys: Vec<String> = calc_ui::keypad().into_iter().flat_map(|row| row.keys).map(|k| k.id).collect();
    let mut vm = ui.view();
    for (index, step) in flow.steps.iter().enumerate() {
        let fail = |why: String| Err(format!("{:?}, step {} {step:?}: {why}", flow.name, index + 1));
        match step {
            Step::Press { id } => {
                if !keys.contains(id) {
                    return fail("no such key on the keypad".to_string());
                }
                vm = ui.press(id);
            }
            Step::Key { key } => match ui.key_input(key) {
                Some(next) => vm = next,
                None => return fail("the calculator ignored it".to_string()),
            },
            Step::Ignored { key } => {
                if let Some(next) = ui.key_input(key) {
                    return fail(format!("the calculator took it: {next:?}"));
                }
            }
            Step::Expect { field, value } => {
                let actual = read(&vm, *field);
                if actual != *value {
                    return fail(format!("it is {actual:?}"));
                }
            }
        }
    }
    Ok(())
}

/// Every flow as JSON, for the front ends' drivers. Written by hand to keep
/// serde out of the repo for one file of test data.
pub fn to_json(flows: &[Flow]) -> String {
    fn string(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }
    let step = |step: &Step| match step {
        Step::Press { id } => format!(r#"{{"op": "press", "id": {}}}"#, string(id)),
        Step::Key { key } => format!(r#"{{"op": "key", "key": {}}}"#, string(key)),
        Step::Ignored { key } => format!(r#"{{"op": "ignored", "key": {}}}"#, string(key)),
        Step::Expect { field, value } => {
            format!(r#"{{"op": "expect", "field": {}, "value": {}}}"#, string(field.name()), string(value))
        }
    };
    let flows: Vec<String> = flows
        .iter()
        .map(|flow| {
            let steps: Vec<String> = flow.steps.iter().map(|s| format!("      {}", step(s))).collect();
            format!(
                "  {{\n    \"name\": {},\n    \"steps\": [\n{}\n    ]\n  }}",
                string(&flow.name),
                steps.join(",\n")
            )
        })
        .collect();
    format!("[\n{}\n]\n", flows.join(",\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_path(name: &str) -> String {
        let dir = std::env::var("TEST_TMPDIR").unwrap_or_else(|_| std::env::temp_dir().display().to_string());
        let path = format!("{dir}/flow-{}-{name}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn every_flow_passes_against_the_rust_view_model() {
        let flows = flows::all();
        assert!(!flows.is_empty());
        let failures: Vec<String> = flows
            .iter()
            .enumerate()
            .filter_map(|(i, flow)| run(flow, &db_path(&i.to_string())).err())
            .collect();
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn flow_names_are_unique() {
        let mut names: Vec<_> = flows::all().into_iter().map(|f| f.name).collect();
        let total = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    #[test]
    fn a_failing_step_is_named() {
        let error = run(&Flow::new("wrong").display("1"), &db_path("wrong")).unwrap_err();
        assert!(error.contains("step 1") && error.contains("\"0\""), "{error}");
        let error = run(&Flow::new("missing").press(&["sqrt"]), &db_path("missing")).unwrap_err();
        assert!(error.contains("no such key"), "{error}");
        let error = run(&Flow::new("tab").key("Tab"), &db_path("tab")).unwrap_err();
        assert!(error.contains("ignored"), "{error}");
    }

    #[test]
    fn steps_export_as_tagged_json() {
        let flow = Flow::new("a \"quoted\"\nname")
            .press(&["6"])
            .key("Enter")
            .ignored("Tab")
            .history(&["1 = 1", "2 = 2"]);
        let json = to_json(&[flow]);
        assert!(json.contains(r#""name": "a \"quoted\"\nname""#), "{json}");
        assert!(json.contains(r#"{"op": "press", "id": "6"}"#), "{json}");
        assert!(json.contains(r#"{"op": "key", "key": "Enter"}"#), "{json}");
        assert!(json.contains(r#"{"op": "ignored", "key": "Tab"}"#), "{json}");
        assert!(json.contains(r#"{"op": "expect", "field": "history", "value": "1 = 1\n2 = 2"}"#), "{json}");
    }
}
