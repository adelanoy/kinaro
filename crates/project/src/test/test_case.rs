use crate::test::{FileTestMetadata, test_step::FileTestStep};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum FileTestCaseType {
  CaseMulti { steps: Vec<FileTestStep> },
  CaseStep {data: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileTestCase {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub case_type: FileTestCaseType,
}
