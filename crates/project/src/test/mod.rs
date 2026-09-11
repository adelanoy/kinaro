use crate::test::test_suite::FileTestSuite;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod test_case;
pub mod test_step;
pub mod test_suite;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestInfo {
  pub id: Uuid,
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disabled: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileTestsContainer {
  pub suites: Vec<FileTestSuite>,
}
