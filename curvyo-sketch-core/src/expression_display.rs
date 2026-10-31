//! Formula display resolves stable references to their current user-facing names.
use crate::{Dimension, Expression, Parameters, SketchError};
use std::fmt::Write;
enum Token<'a> {
    Expression(&'a Expression),
    Text(&'static str),
}
impl Expression {
    /// Formats a parseable expression without changing its stable references.
    /// # Errors
    /// Reports a missing parameter instead of exposing an internal ID as a name.
    pub fn display_text(&self, parameters: &Parameters) -> Result<String, SketchError> {
        let mut output = String::new();
        let mut stack = vec![Token::Expression(self)];
        while let Some(token) = stack.pop() {
            match token {
                Token::Text(text) => output.push_str(text),
                Token::Expression(expression) => match expression {
                    Self::Literal(value) => {
                        let unit = match value.dimension() {
                            Dimension::Scalar => "",
                            Dimension::Length => " mm",
                            Dimension::Angle => " rad",
                        };
                        let _ = write!(output, "{}{unit}", value.value());
                    }
                    Self::Reference(id) => output.push_str(
                        &parameters
                            .0
                            .get(id)
                            .ok_or_else(|| {
                                SketchError::Formula("referenced parameter no longer exists".into())
                            })?
                            .name,
                    ),
                    Self::Negate(value) => {
                        stack.extend([
                            Token::Text(")"),
                            Token::Expression(value),
                            Token::Text("-("),
                        ]);
                    }
                    Self::Add(left, right)
                    | Self::Subtract(left, right)
                    | Self::Multiply(left, right)
                    | Self::Divide(left, right) => {
                        let operator = match expression {
                            Self::Add(..) => " + ",
                            Self::Subtract(..) => " - ",
                            Self::Multiply(..) => " * ",
                            _ => " / ",
                        };
                        stack.extend([
                            Token::Text(")"),
                            Token::Expression(right),
                            Token::Text(operator),
                            Token::Expression(left),
                            Token::Text("("),
                        ]);
                    }
                },
            }
        }
        Ok(output)
    }
}
