use gpui_kit::SharedString;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// A field whose value is either written as-is in the project, or computed by the runtime from an expression.
///
/// Serialized version, untagged, so a literal keeps its plain form (`"duration": 1000`) and an expression is a string
/// (`"duration": "{{var.delay}}"`).
///
/// See [`ExprValue`] for app version
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
#[serde(untagged)]
pub enum SerialExprValue<T> {
  /// A value known when editing the project
  Literal(T),
  /// A value resolved by the runtime when executing the project
  Expr(String),
}

/// A field whose value is either written as-is in the project, or computed by the runtime from an expression.
///
/// App version that holds a SharedString in its Expr variant
///
/// See [`SerialExprValue`] for serialized version
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum ExprValue<T> {
  /// A value known when editing the project
  Literal(T),
  /// A value resolved by the runtime when executing the project
  Expr(SharedString),
}

impl<T: Display> Display for ExprValue<T> {
  /// Writes a literal with its own `Display`, and an expression as its raw text.
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      ExprValue::Literal(value) => value.fmt(f),
      ExprValue::Expr(expr) => f.write_str(expr.as_str()),
    }
  }
}

impl<T> From<T> for ExprValue<T> {
  fn from(value: T) -> Self {
    ExprValue::Literal(value)
  }
}

impl<T: Clone> From<&ExprValue<T>> for SerialExprValue<T> {
  fn from(value: &ExprValue<T>) -> Self {
    match value {
      ExprValue::Literal(v) => SerialExprValue::Literal(v.clone()),
      ExprValue::Expr(expr) => SerialExprValue::Expr(expr.to_string()),
    }
  }
}

impl<T> From<SerialExprValue<T>> for ExprValue<T> {
  fn from(value: SerialExprValue<T>) -> Self {
    match value {
      SerialExprValue::Literal(v) => ExprValue::Literal(v),
      SerialExprValue::Expr(expr) => ExprValue::Expr(expr.into()),
    }
  }
}

impl<T: FromStr> FromStr for ExprValue<T> {
  type Err = T::Err;

  /// Reads `text` as an expression when it starts with `{{` (leading whitespace ignored), as a literal otherwise.
  ///
  /// # Errors
  /// Returns the error of `T::from_str` when `text` is not an expression and not a valid literal
  fn from_str(text: &str) -> Result<Self, Self::Err> {
    let text = text.trim();
    if text.starts_with("{{") {
      Ok(ExprValue::Expr(text.into()))
    } else {
      text.parse().map(ExprValue::Literal)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn literal_is_serialized_as_plain_value() {
    assert_eq!(serde_json::to_string(&SerialExprValue::Literal(1000u32)).unwrap(), "1000");
    assert_eq!(
      serde_json::from_str::<SerialExprValue<u32>>("1000").unwrap(),
      SerialExprValue::Literal(1000)
    );
  }

  #[test]
  fn expression_is_serialized_as_string() {
    let value: SerialExprValue<u32> = SerialExprValue::Expr("{{var.delay}}".into());
    assert_eq!(serde_json::to_string(&value).unwrap(), r#""{{var.delay}}""#);
    assert_eq!(
      serde_json::from_str::<SerialExprValue<u32>>(r#""{{var.delay}}""#).unwrap(),
      value
    );
  }

  #[test]
  fn text_round_trip() {
    assert_eq!(" 42 ".parse::<ExprValue<u32>>().unwrap(), ExprValue::Literal(42));
    assert_eq!(ExprValue::Literal(42u32).to_string(), "42");
    let value = " {{var.delay}}".parse::<ExprValue<u32>>().unwrap();
    assert_eq!(value, ExprValue::Expr("{{var.delay}}".into()));
    assert_eq!(value.to_string(), "{{var.delay}}");
    assert!("abc".parse::<ExprValue<u32>>().is_err());
  }

  #[test]
  fn invalid_literal_is_rejected() {
    assert!(serde_json::from_str::<SerialExprValue<u32>>("-1").is_err());
    assert!(serde_json::from_str::<SerialExprValue<u32>>("true").is_err());
  }
}
