use crate::test::test_step::FileStepData;
use crate::test::{FileTestMetadata, test_step::FileTestStep};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum FileTestCaseType {
  CaseMulti { steps: Vec<FileTestStep> },
  CaseStep { data: FileStepData },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileTestCase {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub case_type: FileTestCaseType,
}

impl FileTestCase {
  /// Returns `true` if this case is a [`FileTestCaseType::CaseStep`].
  pub fn is_case_step(&self) -> bool {
    matches!(self.case_type, FileTestCaseType::CaseStep { .. })
  }
}
