use crate::test::{FileTestInfo, test_step::FileTestStep};
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
  pub info: FileTestInfo,
  pub case_type: FileTestCaseType,
}
