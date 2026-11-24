use regex::Regex;
use std::sync::LazyLock;

static VARIABLE_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[a-zA-Z0-9_ ]+$").unwrap());

/// Returns whether `name` can be used as a variable name: non-empty, and made of ASCII letters, digits, `_` and spaces
pub fn is_variable_name_valid(name: &str) -> bool {
  VARIABLE_NAME_REGEX.is_match(name)
}

#[cfg(test)]
mod tests {
  use super::is_variable_name_valid;

  #[test]
  fn accepts_alphanumeric_and_underscore() {
    for name in ["host", "HOST", "Host", "port8080", "8080", "_", "api_key", "_private", "a"] {
      assert!(is_variable_name_valid(name), "{name:?} should be valid");
    }
  }

  #[test]
  fn accepts_inner_spaces() {
    for name in ["new variable", "new variable_1", "a b c"] {
      assert!(is_variable_name_valid(name), "{name:?} should be valid");
    }
  }

  #[test]
  fn rejects_empty() {
    assert!(!is_variable_name_valid(""));
  }

  #[test]
  fn rejects_punctuation_and_symbols() {
    for name in [
      "my-var", "my.var", "var.host", "{{var}}", "{var}", "a/b", "a$", "a@b", "a+b", "a=b", "a,b", "a:b", "\"a\"",
    ] {
      assert!(!is_variable_name_valid(name), "{name:?} should be rejected");
    }
  }

  #[test]
  fn rejects_whitespace_other_than_space() {
    for name in ["a\tb", "a\nb", "a\r\nb", "a\u{a0}b"] {
      assert!(!is_variable_name_valid(name), "{name:?} should be rejected");
    }
  }

  #[test]
  fn rejects_non_ascii() {
    for name in ["café", "naïve", "変数", "ß", "Ⅷ", "٣"] {
      assert!(!is_variable_name_valid(name), "{name:?} should be rejected");
    }
  }

  #[test]
  fn rejects_trailing_newline() {
    // Guards against `$` matching before a final line break in other regex flavors
    assert!(!is_variable_name_valid("host\n"));
  }
}
