//! Calculator core: a tiny expression evaluator shared by every front end.
//!
//! Grammar (standard precedence, `^` is right-associative):
//!
//! ```text
//! expr   := term (('+' | '-') term)*
//! term   := unary (('*' | '/' | '%') unary)*
//! unary  := '-' unary | power
//! power  := atom ('^' unary)?
//! atom   := number | '(' expr ')'
//! ```

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum CalcError {
    Empty,
    UnexpectedChar { ch: char, pos: usize },
    UnexpectedEnd,
    TrailingInput { pos: usize },
    DivisionByZero,
    NotFinite,
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalcError::Empty => write!(f, "empty expression"),
            CalcError::UnexpectedChar { ch, pos } => write!(f, "unexpected '{ch}' at {pos}"),
            CalcError::UnexpectedEnd => write!(f, "unexpected end of expression"),
            CalcError::TrailingInput { pos } => write!(f, "unexpected input at {pos}"),
            CalcError::DivisionByZero => write!(f, "division by zero"),
            CalcError::NotFinite => write!(f, "result is not a finite number"),
        }
    }
}

impl std::error::Error for CalcError {}

/// Evaluates an arithmetic expression such as `2 * (3 + 4) ^ 2`.
pub fn evaluate(input: &str) -> Result<f64, CalcError> {
    let mut p = Parser { src: input.as_bytes(), pos: 0 };
    p.skip_ws();
    if p.peek().is_none() {
        return Err(CalcError::Empty);
    }
    let value = p.expr()?;
    p.skip_ws();
    if p.peek().is_some() {
        return Err(CalcError::TrailingInput { pos: p.pos });
    }
    if !value.is_finite() {
        return Err(CalcError::NotFinite);
    }
    Ok(value)
}

/// Formats a result for display: integers without a decimal point, other
/// values with at most 10 significant fractional digits and no trailing zeros.
pub fn format_number(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let s = format!("{value:.10}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n')) {
            self.pos += 1;
        }
    }

    /// Consumes `op` if it is the next non-whitespace byte.
    fn eat(&mut self, op: u8) -> bool {
        self.skip_ws();
        if self.peek() == Some(op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Result<f64, CalcError> {
        let mut acc = self.term()?;
        loop {
            if self.eat(b'+') {
                acc += self.term()?;
            } else if self.eat(b'-') {
                acc -= self.term()?;
            } else {
                return Ok(acc);
            }
        }
    }

    fn term(&mut self) -> Result<f64, CalcError> {
        let mut acc = self.unary()?;
        loop {
            if self.eat(b'*') {
                acc *= self.unary()?;
            } else if self.eat(b'/') {
                let rhs = self.unary()?;
                if rhs == 0.0 {
                    return Err(CalcError::DivisionByZero);
                }
                acc /= rhs;
            } else if self.eat(b'%') {
                let rhs = self.unary()?;
                if rhs == 0.0 {
                    return Err(CalcError::DivisionByZero);
                }
                acc %= rhs;
            } else {
                return Ok(acc);
            }
        }
    }

    fn unary(&mut self) -> Result<f64, CalcError> {
        if self.eat(b'-') {
            return Ok(-self.unary()?);
        }
        self.power()
    }

    fn power(&mut self) -> Result<f64, CalcError> {
        let base = self.atom()?;
        if self.eat(b'^') {
            return Ok(base.powf(self.unary()?));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<f64, CalcError> {
        self.skip_ws();
        match self.peek() {
            None => Err(CalcError::UnexpectedEnd),
            Some(b'(') => {
                self.pos += 1;
                let v = self.expr()?;
                if !self.eat(b')') {
                    return match self.peek() {
                        None => Err(CalcError::UnexpectedEnd),
                        Some(c) => Err(CalcError::UnexpectedChar { ch: c as char, pos: self.pos }),
                    };
                }
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => self.number(),
            Some(c) => Err(CalcError::UnexpectedChar { ch: c as char, pos: self.pos }),
        }
    }

    fn number(&mut self) -> Result<f64, CalcError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == b'.') {
            self.pos += 1;
        }
        // The slice is ASCII digits and dots, so it is valid UTF-8.
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        text.parse::<f64>().map_err(|_| CalcError::UnexpectedChar { ch: '.', pos: start })
    }
}

/// One evaluated line, as kept in [`Calculator`] history.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub expression: String,
    pub result: String,
}

/// A stateful calculator that remembers successful evaluations and exposes
/// the previous answer as `ans`.
#[derive(Debug, Default)]
pub struct Calculator {
    history: Vec<Entry>,
    last: Option<f64>,
}

impl Calculator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates `input`, substituting `ans` with the previous result, and
    /// records it in history on success. Returns the formatted result.
    pub fn evaluate(&mut self, input: &str) -> Result<String, CalcError> {
        let expanded = match self.last {
            Some(v) => input.replace("ans", &format!("({v})")),
            None => input.to_string(),
        };
        let value = evaluate(&expanded)?;
        let result = format_number(value);
        self.last = Some(value);
        self.history.push(Entry { expression: input.trim().to_string(), result: result.clone() });
        Ok(result)
    }

    pub fn history(&self) -> &[Entry] {
        &self.history
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_and_parens() {
        assert_eq!(evaluate("1 + 2 * 3").unwrap(), 7.0);
        assert_eq!(evaluate("(1 + 2) * 3").unwrap(), 9.0);
        assert_eq!(evaluate("2 ^ 3 ^ 2").unwrap(), 512.0);
        assert_eq!(evaluate("-2 ^ 2").unwrap(), -4.0);
        assert_eq!(evaluate("10 % 4").unwrap(), 2.0);
        assert_eq!(evaluate(" .5 + 1.25 ").unwrap(), 1.75);
    }

    #[test]
    fn errors() {
        assert_eq!(evaluate(""), Err(CalcError::Empty));
        assert_eq!(evaluate("1 / 0"), Err(CalcError::DivisionByZero));
        assert_eq!(evaluate("(1 + 2"), Err(CalcError::UnexpectedEnd));
        assert_eq!(evaluate("1 + x"), Err(CalcError::UnexpectedChar { ch: 'x', pos: 4 }));
        assert_eq!(evaluate("1 2"), Err(CalcError::TrailingInput { pos: 2 }));
    }

    #[test]
    fn formatting() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
        assert_eq!(format_number(-1.5), "-1.5");
    }

    #[test]
    fn calculator_history_and_ans() {
        let mut c = Calculator::new();
        assert_eq!(c.evaluate("6 * 7").unwrap(), "42");
        assert_eq!(c.evaluate("ans / 2").unwrap(), "21");
        assert!(c.evaluate("1 / 0").is_err());
        assert_eq!(c.history().len(), 2);
        c.clear();
        assert!(c.history().is_empty());
    }
}
