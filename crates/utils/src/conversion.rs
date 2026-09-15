/// Splits a multiline string into its individual lines, for storage as a
/// `Vec<String>` in a line-oriented format (e.g. JSON) instead of a single
/// string with embedded `\n`s, so that version control diffs only the lines
/// that actually changed.
///
/// A trailing newline is not preserved: `"a\nb\n"` and `"a\nb"` both produce
/// `["a", "b"]`. Use [`from_multiline`] to rejoin the result.
pub fn to_multiline(s: &str) -> Vec<String> {
  s.lines().map(String::from).collect()
}

/// Rejoins lines produced by [`to_multiline`] back into a single string,
/// or `None` if there were no lines.
pub fn from_multiline(lines: &[String]) -> Option<String> {
  if lines.is_empty() {
    None
  } else {
    Some(lines.join("\n"))
  }
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
  fn to_multiline_drops_a_trailing_newline() {
    assert_eq!(to_multiline("a\nb\n"), vec!["a", "b"]);
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
    let original = "line one\nline two\nline three";
    assert_eq!(from_multiline(&to_multiline(original)), Some(original.to_string()));
  }
}
