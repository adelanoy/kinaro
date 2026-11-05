/// Splits a multiline string on `\n` into its individual lines, for storage as a
/// `Vec<String>` in a line-oriented format (e.g. JSON) instead of a single
/// string with embedded `\n`s, so that version control diffs only the lines
/// that actually changed.
///
/// The split is lossless: trailing newlines become trailing empty lines
/// (`"a\n"` produces `["a", ""]`), `\r` characters are kept as-is, and an empty
/// string produces no lines. Use [`from_multiline`] to rejoin the result.
pub fn to_multiline(s: &str) -> Vec<String> {
  if s.is_empty() {
    vec![]
  } else {
    s.split('\n').map(String::from).collect()
  }
}

/// Rejoins lines produced by [`to_multiline`] back into a single string,
/// or `None` if there were no lines.
pub fn from_multiline(lines: &[String]) -> Option<String> {
  if lines.is_empty() { None } else { Some(lines.join("\n")) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_multiline_splits_on_newlines() {
    assert_eq!(to_multiline("a\nb\nc"), vec!["a", "b", "c"]);
  }

  #[test]
  fn to_multiline_of_empty_string_is_empty() {
    assert!(to_multiline("").is_empty());
  }

  #[test]
  fn to_multiline_keeps_trailing_newlines_as_empty_lines() {
    assert_eq!(to_multiline("a\nb\n"), vec!["a", "b", ""]);
    assert_eq!(to_multiline("a\n\n"), vec!["a", "", ""]);
    assert_eq!(to_multiline("\n"), vec!["", ""]);
  }

  #[test]
  fn to_multiline_keeps_carriage_returns() {
    assert_eq!(to_multiline("a\r\nb"), vec!["a\r", "b"]);
  }

  #[test]
  fn from_multiline_of_no_lines_is_none() {
    assert_eq!(from_multiline(&[]), None);
  }

  #[test]
  fn from_multiline_joins_with_newlines() {
    let lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert_eq!(from_multiline(&lines), Some("a\nb\nc".to_string()));
  }

  #[test]
  fn round_trips_through_to_then_from() {
    for original in [
      "line one\nline two\nline three",
      "a",
      "a\n",
      "a\n\n",
      "\n",
      "\n\n",
      "\na",
      "a\r\nb\r\n",
    ] {
      assert_eq!(
        from_multiline(&to_multiline(original)),
        Some(original.to_string()),
        "round trip of {original:?}"
      );
    }
  }

  #[test]
  fn empty_string_round_trips_to_none() {
    assert_eq!(from_multiline(&to_multiline("")), None);
  }
}
