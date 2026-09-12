use crate::test::{TestInfo, test_case::TestCase};
use ki_project::FileTestSuite;
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Clone, Debug, Eq)]
pub struct TestSuite {
  pub info: TestInfo,
  pub cases: Vec<TestCase>,
}

impl TestSuite {
  pub fn from_file(file_test_suites: Vec<FileTestSuite>) -> Vec<TestSuite> {
    file_test_suites
      .into_iter()
      .map(|file_test_suite| Self {
        info: TestInfo::from_file(file_test_suite.info),
        cases: TestCase::from_file(file_test_suite.cases),
      })
      .collect()
  }

  pub fn get_file(&self) -> FileTestSuite {
    FileTestSuite {
      info: self.info.get_file(),
      cases: self.cases.iter().map(|case| case.get_file()).collect(),
    }
  }

  #[allow(unused)]
  pub fn info_from_path(&self, path: &[Uuid]) -> Option<&TestInfo> {
    let case = self.cases.iter().find(|case| case.info.id == path[0])?;
    if path.len() == 1 {
      Some(&case.info)
    } else {
      case.info_from_path(&path[1])
    }
  }

  pub fn info_mut_from_path(&mut self, path: &[Uuid]) -> Option<&mut TestInfo> {
    let case = self.cases.iter_mut().find(|case| case.info.id == path[0])?;
    if path.len() == 1 {
      Some(&mut case.info)
    } else {
      case.info_mut_from_path(&path[1])
    }
  }
}

impl PartialEq for TestSuite {
  fn eq(&self, other: &Self) -> bool {
    self.info.name == other.info.name && self.cases == other.cases
  }
}
