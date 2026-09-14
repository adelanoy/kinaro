use crate::test::{TestInfo, TestInfoId};
use gpui_kit::SharedString;
use ki_project::FileTestStep;
use uuid::Uuid;

#[derive(Clone, Debug, Eq)]
pub struct TestStep {
  pub info: TestInfo,
  pub data: String,
}

impl TestStep {
  pub(super) fn duplicate(&self, name: SharedString) -> Self {
    Self {
      info: self.info.duplicate(name),
      data: "".to_string(),
    }
  }
  
  pub fn from_file(file_test_step: FileTestStep, suite_id: Uuid, case_id: Uuid) -> TestStep {
    Self {
        info: TestInfo::new_step(file_test_step.info, suite_id, case_id),
        data: file_test_step.data.clone(),
      }
  }
  pub fn get_file(&self) -> FileTestStep {
    FileTestStep {
      info: self.info.to_file(),
      data: self.data.clone(),
    }
  }
  
  pub fn new(suite_id: Uuid, case_id: Uuid, name: SharedString) -> Self {
    Self {
      info: TestInfo {
        info_id: TestInfoId::Step(suite_id, case_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      data: "".to_string(),
    }
  }
}

impl PartialEq for TestStep {
  fn eq(&self, other: &Self) -> bool {
    self.info.info_id == other.info.info_id || self.info.name == other.info.name
  }
}
