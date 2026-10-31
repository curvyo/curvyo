//! Formula parsing resolves names with an iterative operator stack.
use crate::{Dimension, Expression, Parameters, Quantity, SketchError};

pub(crate) fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}
#[derive(Clone, Copy, PartialEq)]
enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,
    Positive,
    Open,
}
impl Operator {
    fn precedence(self) -> u8 {
        match self {
            Self::Open => 0,
            Self::Add | Self::Subtract => 1,
            Self::Multiply | Self::Divide => 2,
            Self::Negate | Self::Positive => 3,
        }
    }
}
pub(crate) fn parse(text: &str, parameters: &Parameters) -> Result<Expression, SketchError> {
    let mut lexer = Lexer {
        text,
        offset: 0,
        parameters,
    };
    let mut values = vec![];
    let mut ops = vec![];
    let mut operand = true;
    while {
        lexer.space();
        lexer.peek().is_some()
    } {
        let c = lexer.peek().ok_or_else(|| lexer.error("missing token"))?;
        if operand {
            if c == '(' {
                ops.push(Operator::Open);
                lexer.step();
            } else if c == '-' || c == '+' {
                ops.push(if c == '-' {
                    Operator::Negate
                } else {
                    Operator::Positive
                });
                lexer.step();
            } else {
                values.push(lexer.atom()?);
                operand = false;
            }
        } else if c == ')' {
            lexer.step();
            while ops.last().is_some_and(|o| *o != Operator::Open) {
                apply(&mut ops, &mut values)?;
            }
            if ops.pop() != Some(Operator::Open) {
                return Err(lexer.error("unmatched closing parenthesis"));
            }
        } else {
            let op = match c {
                '+' => Operator::Add,
                '-' => Operator::Subtract,
                '*' => Operator::Multiply,
                '/' => Operator::Divide,
                _ => return Err(lexer.error("expected operator")),
            };
            lexer.step();
            while ops
                .last()
                .is_some_and(|old| old.precedence() >= op.precedence())
            {
                apply(&mut ops, &mut values)?;
            }
            ops.push(op);
            operand = true;
        }
    }
    if operand {
        return Err(lexer.error("missing operand"));
    }
    while !ops.is_empty() {
        apply(&mut ops, &mut values)?;
    }
    if values.len() != 1 {
        return Err(lexer.error("invalid expression"));
    }
    values.pop().ok_or_else(|| lexer.error("empty expression"))
}
fn apply(ops: &mut Vec<Operator>, values: &mut Vec<Expression>) -> Result<(), SketchError> {
    let invalid = || SketchError::Formula("missing operand or closing parenthesis".into());
    let op = ops.pop().ok_or_else(invalid)?;
    let right = values.pop().ok_or_else(invalid)?;
    if op == Operator::Open {
        return Err(invalid());
    }
    if op == Operator::Positive {
        values.push(right);
        return Ok(());
    }
    if op == Operator::Negate {
        values.push(Expression::Negate(Box::new(right)));
        return Ok(());
    }
    let left = Box::new(values.pop().ok_or_else(invalid)?);
    let right = Box::new(right);
    values.push(match op {
        Operator::Add => Expression::Add(left, right),
        Operator::Subtract => Expression::Subtract(left, right),
        Operator::Multiply => Expression::Multiply(left, right),
        Operator::Divide => Expression::Divide(left, right),
        _ => return Err(invalid()),
    });
    Ok(())
}
struct Lexer<'a> {
    text: &'a str,
    offset: usize,
    parameters: &'a Parameters,
}
impl Lexer<'_> {
    fn error(&self, message: &str) -> SketchError {
        SketchError::Formula(format!("{message} at byte {}", self.offset))
    }
    fn peek(&self) -> Option<char> {
        self.text[self.offset..].chars().next()
    }
    fn step(&mut self) {
        if let Some(c) = self.peek() {
            self.offset += c.len_utf8();
        }
    }
    fn space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.step();
        }
    }
    fn atom(&mut self) -> Result<Expression, SketchError> {
        if self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
            return self.number();
        }
        let start = self.offset;
        while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.step();
        }
        let name = &self.text[start..self.offset];
        if !valid_name(name) {
            return Err(self.error("invalid parameter name"));
        }
        let mut matches = self.parameters.0.iter().filter(|(_, p)| p.name == name);
        let (id, _) = matches
            .next()
            .ok_or_else(|| self.error(&format!("unknown parameter {name}")))?;
        if matches.next().is_some() {
            return Err(self.error("ambiguous parameter name"));
        }
        Ok(Expression::Reference(id.clone()))
    }
    fn number(&mut self) -> Result<Expression, SketchError> {
        let start = self.offset;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
            self.step();
        }
        if self.peek().is_some_and(|c| c == 'e' || c == 'E') {
            self.step();
            if self.peek().is_some_and(|c| c == '+' || c == '-') {
                self.step();
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.step();
            }
        }
        let value: f64 = self.text[start..self.offset]
            .parse()
            .map_err(|_| self.error("invalid number"))?;
        self.space();
        let unit_start = self.offset;
        while self.peek().is_some_and(|c| c.is_alphabetic() || c == '°') {
            self.step();
        }
        let (scale, dimension) = match &self.text[unit_start..self.offset] {
            "" => (1.0, Dimension::Scalar),
            "mm" => (1.0, Dimension::Length),
            "cm" => (10.0, Dimension::Length),
            "m" => (1000.0, Dimension::Length),
            "in" => (25.4, Dimension::Length),
            "deg" | "°" => (std::f64::consts::PI / 180.0, Dimension::Angle),
            "rad" => (1.0, Dimension::Angle),
            _ => return Err(self.error("unknown unit")),
        };
        Ok(Expression::Literal(Quantity::new(
            value * scale,
            dimension,
        )?))
    }
}
