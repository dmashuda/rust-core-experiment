//! The flows. Add one here and every front end's test runs it.

use crate::Flow;

pub fn all() -> Vec<Flow> {
    vec![
        evaluating(),
        history_and_ans(),
        errors(),
        clear_and_backspace(),
        continuing_from_the_answer(),
        the_keyboard(),
    ]
}

fn evaluating() -> Flow {
    Flow::new("keys build an expression and = evaluates it")
        .expression("")
        .display("0")
        .error(false)
        .history(&[])
        .press(&["2", "*", "(", "3", "+", "4", ")", "^", "2"])
        .expression("2*(3+4)^2")
        .display("0")
        .press(&["evaluate"])
        .expression("")
        .display("98")
        .error(false)
        .history(&["2*(3+4)^2 = 98"])
        .press(&["7", "%", "4", "+", "0", ".", "5", "evaluate"])
        .display("3.5")
}

fn history_and_ans() -> Flow {
    Flow::new("history is newest first, and ans is the last answer")
        .press(&["6", "*", "7", "evaluate"])
        .press(&["ans", "/", "2"])
        .expression("ans/2")
        .press(&["evaluate"])
        .display("21")
        .history(&["ans/2 = 21", "6*7 = 42"])
}

fn errors() -> Flow {
    Flow::new("an error keeps the expression, and editing it brings back the last result")
        .press(&["1", "/", "0", "evaluate"])
        .error(true)
        .display("division by zero")
        .expression("1/0")
        .history(&[])
        .press(&["backspace"])
        .expression("1/")
        .error(false)
        .display("0")
        .press(&["clear", "6", "*", "7", "evaluate"])
        .press(&["1", "/", "0", "evaluate"])
        .error(true)
        .press(&["5"])
        .error(false)
        .display("42")
        .history(&["6*7 = 42"])
}

fn clear_and_backspace() -> Flow {
    Flow::new("clear and backspace edit the expression; clear keeps history and ans")
        .press(&["1", "2", "backspace"])
        .expression("1")
        .press(&["clear"])
        .expression("")
        .press(&["backspace"])
        .expression("")
        .press(&["6", "*", "7", "evaluate", "clear"])
        .display("0")
        .history(&["6*7 = 42"])
        .press(&["ans", "evaluate"])
        .display("42")
}

fn continuing_from_the_answer() -> Flow {
    Flow::new("an operator after = continues from the answer; a digit starts over")
        .press(&["+"])
        .expression("+")
        .press(&["clear", "6", "*", "7", "evaluate", "+"])
        .expression("ans+")
        .press(&["1", "evaluate"])
        .display("43")
        .press(&["5"])
        .expression("5")
}

fn the_keyboard() -> Flow {
    Flow::new("a hardware keyboard types, evaluates, deletes and clears")
        .typed("6*7")
        .expression("6*7")
        .key("Enter")
        .display("42")
        .typed("*")
        .expression("ans*")
        .key("Escape")
        .expression("")
        .display("0")
        .typed("ans+1")
        .key("=")
        .display("43")
        .typed("12")
        .key("Backspace")
        .expression("1")
        .ignored("Tab")
        .ignored("A")
        .expression("1")
        .key("Escape")
        .key("Enter")
        .error(true)
}
