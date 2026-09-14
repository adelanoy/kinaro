use crate::error::ProjectError::TestNotFound;
use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestInfo, TestInfoId, test_step::TestStep};
use gpui_kit::SharedString;
use ki_project::{FileTestCase, FileTestCaseType};
use log::error;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestCaseType {
  Case { steps: Vec<TestStep> },
  CaseStep { data: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
  pub info: TestInfo,
  case_type: TestCaseType,
}

impl TestCase {
  #[inline]
  pub fn is_step(&self) -> bool {
    matches!(self.case_type, TestCaseType::CaseStep { .. })
  }

  #[inline]
  pub fn case_type(&self) -> &TestCaseType {
    &self.case_type
  }

  pub(crate) fn from_file(file_test_case: FileTestCase, suite_id: Uuid) -> Self {
    let id = file_test_case.info.id;
    match file_test_case.case_type {
      FileTestCaseType::Case { steps } => Self {
        info: TestInfo::new_case(file_test_case.info, suite_id),
        case_type: TestCaseType::Case {
          steps: steps
            .into_iter()
            .map(|step| TestStep::from_file(step, suite_id, id))
            .collect(),
        },
      },
      FileTestCaseType::CaseStep { data } => Self {
        info: TestInfo::new_case_step(file_test_case.info, suite_id),
        case_type: TestCaseType::CaseStep { data },
      },
    }
  }

  pub(crate) fn get_file(&self) -> FileTestCase {
    FileTestCase {
      info: self.info.to_file(),
      case_type: match &self.case_type {
        TestCaseType::Case { steps } => FileTestCaseType::Case {
          steps: steps.iter().map(|step| step.get_file()).collect(),
        },
        TestCaseType::CaseStep { data } => FileTestCaseType::CaseStep { data: data.clone() },
      },
    }
  }

  #[allow(unused)]
  pub fn info_from_path(&self, info_id: &TestInfoId) -> Option<&TestInfo> {
    match &self.case_type {
      TestCaseType::Case { steps } => steps.iter().find(|step| step.info.info_id == *info_id).map(|step| &step.info),
      TestCaseType::CaseStep { .. } => None,
    }
  }

  pub fn info_mut_from_path(&mut self, info_id: &TestInfoId) -> Option<&mut TestInfo> {
    match &mut self.case_type {
      TestCaseType::Case { steps } => steps
        .iter_mut()
        .find(|step| step.info.info_id == *info_id)
        .map(|step| &mut step.info),
      TestCaseType::CaseStep { .. } => None,
    }
  }

  /// Duplicates this TestCase with a new random ID and the given name. All children are also duplicated with a new ID, but keep their
  /// original name, whether this is a regular or a step case
  pub(super) fn duplicate(&self, name: SharedString) -> Self {
    Self {
      info: self.info.duplicate(name),
      case_type: match &self.case_type {
        TestCaseType::Case { steps } => TestCaseType::Case {
          steps: steps.iter().map(|step| step.duplicate(step.info.name.clone())).collect(),
        },
        TestCaseType::CaseStep { data } => TestCaseType::CaseStep { data: data.clone() },
      },
    }
  }

  /// Duplicates this TestCase's children steps.
  ///
  /// # Result
  /// This cannot be called on a Step Case and will return a
  pub(crate) fn duplicate_child(&mut self, info_id: &TestInfoId) -> ProjectResult<()> {
    match &mut self.case_type {
      TestCaseType::Case { steps } => {
        let Some(ix) = steps.iter().position(|step| step.info.info_id == *info_id) else {
          error!("TestCase:duplicate_child: unknow path: {}", info_id);
          return Err(TestNotFound);
        };
        let name = ki_utils::next_available_name(&steps[ix].info.name, steps.iter().map(|case| case.info.name.clone()));
        let duplicate = steps[ix].duplicate(name);
        if ix == steps.len() - 1 {
          steps.push(duplicate);
        } else {
          steps.insert(ix + 1, duplicate);
        }
        Ok(())
      }
      TestCaseType::CaseStep { .. } => {
        error!(
          "TestCase:duplicate_child: cannot duplicated children on a step case. path: {}",
          info_id
        );
        Err(ProjectError::OperationNotAllowed)
      }
    }
  }
}
