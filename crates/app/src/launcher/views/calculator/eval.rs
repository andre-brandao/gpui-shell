//! Arithmetic over `+ - * / % ^`, parentheses, unary minus, `pi` and `e`,
//! reduced one operation at a time so each step can be shown.

/// The reductions from an expression to its value. On an error midway the
/// steps up to it are kept.
#[derive(Debug, PartialEq)]
pub struct Evaluation {
    /// Each intermediate expression, without the input and the result.
    pub steps: Vec<String>,
    pub result: Result<f64, &'static str>,
}

pub fn evaluate(input: &str) -> Evaluation {
    let mut parser = Parser {
        chars: input.chars().filter(|c| !c.is_whitespace()).collect(),
        pos: 0,
    };
    let mut expr = match parser.parse() {
        Ok(expr) => expr,
        Err(err) => {
            return Evaluation {
                steps: Vec::new(),
                result: Err(err),
            };
        }
    };

    let mut steps: Vec<String> = Vec::new();
    let result = loop {
        match expr.reduce() {
            Ok(true) => {
                let step = expr.to_string();
                // Folding a sign or a constant can print the same text.
                if steps.last() != Some(&step) {
                    steps.push(step);
                }
            }
            Ok(false) => break Ok(expr.value()),
            Err(err) => break Err(err),
        }
    };
    if let Ok(value) = result
        && steps.last() == Some(&format_number(value))
    {
        steps.pop();
    }
    Evaluation { steps, result }
}

/// `value` without float noise: `14`, `0.3`.
pub fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.10}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
}

impl Op {
    fn precedence(self) -> u8 {
        match self {
            Op::Add | Op::Sub => 1,
            Op::Mul | Op::Div | Op::Rem => 2,
            Op::Pow => 4,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::Rem => "%",
            Op::Pow => "^",
        }
    }

    fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            Op::Add => a + b,
            Op::Sub => a - b,
            Op::Mul => a * b,
            Op::Div => a / b,
            Op::Rem => a % b,
            Op::Pow => a.powf(b),
        }
    }
}

/// Unary minus sits between `*` and `^`: `-2 ^ 2` is `-(2 ^ 2)`.
const NEG_PRECEDENCE: u8 = 3;
const ATOM_PRECEDENCE: u8 = 5;

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Num(f64),
    Const(&'static str, f64),
    Neg(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
}

impl Expr {
    fn precedence(&self) -> u8 {
        match self {
            Expr::Num(value) if *value < 0.0 => NEG_PRECEDENCE,
            Expr::Num(_) | Expr::Const(..) => ATOM_PRECEDENCE,
            Expr::Neg(_) => NEG_PRECEDENCE,
            Expr::Bin(op, ..) => op.precedence(),
        }
    }

    fn value(&self) -> f64 {
        match self {
            Expr::Num(value) => *value,
            _ => unreachable!("an irreducible expression is a number"),
        }
    }

    /// Reduce the leftmost innermost operation. `false` once a number.
    fn reduce(&mut self) -> Result<bool, &'static str> {
        let value = match self {
            Expr::Num(_) => return Ok(false),
            Expr::Const(_, value) => *value,
            Expr::Neg(inner) => {
                if inner.reduce()? {
                    return Ok(true);
                }
                let Expr::Num(value) = **inner else {
                    unreachable!()
                };
                -value
            }
            Expr::Bin(op, left, right) => {
                if left.reduce()? || right.reduce()? {
                    return Ok(true);
                }
                let (Expr::Num(a), Expr::Num(b)) = (&**left, &**right) else {
                    unreachable!()
                };
                op.apply(*a, *b)
            }
        };
        if !value.is_finite() {
            return Err("Not a finite number");
        }
        *self = Expr::Num(value);
        Ok(true)
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let wrap = |f: &mut std::fmt::Formatter<'_>, expr: &Expr, parens: bool| {
            if parens {
                write!(f, "({expr})")
            } else {
                write!(f, "{expr}")
            }
        };
        match self {
            Expr::Num(value) => f.write_str(&format_number(*value)),
            Expr::Const(name, _) => f.write_str(name),
            Expr::Neg(inner) => {
                f.write_str("-")?;
                wrap(f, inner, inner.precedence() <= NEG_PRECEDENCE)
            }
            Expr::Bin(op, left, right) => {
                let p = op.precedence();
                // `^` groups to the right, the rest to the left.
                let (left_parens, right_parens) = if *op == Op::Pow {
                    (left.precedence() <= p, right.precedence() < p)
                } else {
                    (left.precedence() < p, right.precedence() <= p)
                };
                wrap(f, left, left_parens)?;
                write!(f, " {} ", op.symbol())?;
                wrap(f, right, right_parens)
            }
        }
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn parse(&mut self) -> Result<Expr, &'static str> {
        let expr = self.sum()?;
        if self.pos != self.chars.len() {
            return Err("Unexpected input");
        }
        Ok(expr)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        let found = self.peek() == Some(c);
        if found {
            self.pos += 1;
        }
        found
    }

    fn binary(op: Op, left: Expr, right: Expr) -> Expr {
        Expr::Bin(op, Box::new(left), Box::new(right))
    }

    fn sum(&mut self) -> Result<Expr, &'static str> {
        let mut expr = self.product()?;
        loop {
            let op = if self.eat('+') {
                Op::Add
            } else if self.eat('-') {
                Op::Sub
            } else {
                return Ok(expr);
            };
            expr = Self::binary(op, expr, self.product()?);
        }
    }

    fn product(&mut self) -> Result<Expr, &'static str> {
        let mut expr = self.unary()?;
        loop {
            let op = if self.eat('*') || self.eat('x') {
                Op::Mul
            } else if self.eat('/') {
                Op::Div
            } else if self.eat('%') {
                Op::Rem
            } else {
                return Ok(expr);
            };
            expr = Self::binary(op, expr, self.unary()?);
        }
    }

    fn unary(&mut self) -> Result<Expr, &'static str> {
        if self.eat('-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.eat('+');
        self.power()
    }

    fn power(&mut self) -> Result<Expr, &'static str> {
        let base = self.atom()?;
        if self.eat('^') {
            return Ok(Self::binary(Op::Pow, base, self.unary()?));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<Expr, &'static str> {
        if self.eat('(') {
            let expr = self.sum()?;
            return if self.eat(')') {
                Ok(expr)
            } else {
                Err("Missing )")
            };
        }
        let rest: String = self.chars[self.pos..].iter().collect();
        for (name, value) in [("pi", std::f64::consts::PI), ("e", std::f64::consts::E)] {
            if rest.starts_with(name) {
                self.pos += name.len();
                return Ok(Expr::Const(name, value));
            }
        }
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || c == '.' || c == ',')
        {
            self.pos += 1;
        }
        let number: String = self.chars[start..self.pos]
            .iter()
            .filter(|c| **c != ',')
            .collect();
        number.parse().map(Expr::Num).map_err(|_| "Not a number")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(input: &str) -> Result<f64, &'static str> {
        evaluate(input).result
    }

    #[test]
    fn precedence_and_parentheses() {
        assert_eq!(value("2 + 3 * 4"), Ok(14.0));
        assert_eq!(value("(2 + 3) * 4"), Ok(20.0));
        assert_eq!(value("2 ^ 3 ^ 2"), Ok(512.0));
        assert_eq!(value("-2 ^ 2"), Ok(-4.0));
        assert_eq!(value("10 % 4"), Ok(2.0));
        assert_eq!(value("10 - 4 - 3"), Ok(3.0));
    }

    #[test]
    fn constants_and_thousands_separators() {
        assert_eq!(value("1,000 * 2"), Ok(2000.0));
        assert!((value("2 * pi").unwrap() - std::f64::consts::TAU).abs() < 1e-12);
    }

    #[test]
    fn errors_are_reported() {
        assert_eq!(value("(1 + 2"), Err("Missing )"));
        assert_eq!(value("1 +"), Err("Not a number"));
        assert_eq!(value("1 / 0"), Err("Not a finite number"));
    }

    #[test]
    fn steps_reduce_one_operation_at_a_time() {
        assert_eq!(evaluate("2 * (3 + 4) - 1").steps, ["2 * 7 - 1", "14 - 1"]);
        assert_eq!(evaluate("(1+2)*(3+4)").steps, ["3 * (3 + 4)", "3 * 7"]);
        assert!(evaluate("42").steps.is_empty());
        assert!(evaluate("-2 ^ 2").steps.is_empty());
    }

    #[test]
    fn steps_keep_grouping_and_signs() {
        assert_eq!(evaluate("10 - (4 - 3)").steps, ["10 - 1"]);
        assert_eq!(evaluate("(0 - 3) ^ 2 + 1").steps, ["(-3) ^ 2 + 1", "9 + 1"]);
        assert_eq!(evaluate("-(1 + 2) * 2").steps, ["-3 * 2"]);
    }

    #[test]
    fn steps_stop_at_an_error() {
        let eval = evaluate("1 + 2 / (3 - 3)");
        assert_eq!(eval.steps, ["1 + 2 / 0"]);
        assert_eq!(eval.result, Err("Not a finite number"));
    }

    #[test]
    fn results_are_formatted_without_noise() {
        assert_eq!(format_number(14.0), "14");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
    }
}
