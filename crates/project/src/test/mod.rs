use crate::test::test_suite::FileTestSuite;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod test_case;
pub mod test_step;
pub mod test_suite;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestMetadata {
  pub id: Uuid,
  pub name: String,
  /// One entry per line, so version control diffs only the lines that
  /// actually changed instead of one escaped multiline string.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub description: Vec<String>,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disabled: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileTestsContainer {
  pub suites: Vec<FileTestSuite>,
}
