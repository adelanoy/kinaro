use crate::error::{ProjectError::TestNotFound, ProjectResult};
use crate::test::{TestInfo, TestInfoId, test_case::TestCase};
use gpui_kit::SharedString;
use ki_project::FileTestSuite;
use log::{error, warn};
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSuite {
  pub info: TestInfo,
  pub cases: Vec<TestCase>,
}

impl TestSuite {
  pub fn add_test_case(&mut self) -> ProjectResult<()> {
    let name = ki_utils::next_available_name("new Case", self.cases.iter().map(|case| case.info.name.clone()));
    self.cases.push(TestCase::new(self.info.id(), name));
    Ok(())
  }

  pub fn add_test_step(&mut self, info_id: &TestInfoId) -> ProjectResult<()> {
    match info_id {
      TestInfoId::Suite(_) | TestInfoId::CaseStep(_, _) => {
        let name = ki_utils::next_available_name("new Test", self.cases.iter().map(|case| case.info.name.clone()));
        self.cases.push(TestCase::new_case_step(self.info.id(), name));
        Ok(())
      }
      TestInfoId::CaseMulti(_, case_id) | TestInfoId::Step(_, case_id, _) => {
        let Some(case) = self.cases.iter_mut().find(|case| case.info.id() == *case_id) else {
          warn!("TestSuite:add_test_step: unknow path: {}", info_id);
          return Err(TestNotFound);
        };
        case.add_test_step(self.info.id())
      }
    }
  }

  pub(crate) fn duplicate(&self, name: SharedString) -> Self {
    Self {
      info: self.info.duplicate(name),
      cases: self.cases.iter().map(|tc| tc.duplicate(tc.info.name.clone())).collect(),
    }
  }

  pub(crate) fn duplicate_child(&mut self, info_id: &TestInfoId) -> ProjectResult<()> {
    let Some(ix) = self.cases.iter().position(|case| case.info.info_id.is_parent(info_id)) else {
      error!("TestSuite:duplicate_child: unknow path: {}", info_id);
      return Err(TestNotFound);
    };

    if matches!(info_id, TestInfoId::CaseMulti(_, _)) || self.cases[ix].is_step() {
      let name = ki_utils::next_available_name(
        &self.cases[ix].info.name,
        self.cases.iter().map(|case| case.info.name.clone()),
      );
      let duplicate = self.cases[ix].duplicate(name);
      if ix == self.cases.len() - 1 {
        self.cases.push(duplicate);
      } else {
        self.cases.insert(ix + 1, duplicate);
      }
      Ok(())
    } else {
      self.cases[ix].duplicate_child(info_id)
    }
  }

  pub(crate) fn from_file(file_test_suite: FileTestSuite) -> TestSuite {
    let id = file_test_suite.info.id;
    Self {
      info: TestInfo::new_suite(file_test_suite.info),
      cases: file_test_suite
        .cases
        .into_iter()
        .map(|case| TestCase::from_file(case, id))
        .collect(),
    }
  }

  pub(crate) fn get_file(&self) -> FileTestSuite {
    FileTestSuite {
      info: self.info.to_file(),
      cases: self.cases.iter().map(|case| case.get_file()).collect(),
    }
  }

  #[allow(unused)]
  pub fn info_from_path(&self, info_id: &TestInfoId) -> Option<&TestInfo> {
    let case = self.cases.iter().find(|case| case.info.info_id == *info_id)?;
    match info_id {
      TestInfoId::Suite(_) => None,
      TestInfoId::CaseMulti(_, _) | TestInfoId::CaseStep(_, _) => Some(&case.info),
      TestInfoId::Step(_, _, _) => case.info_from_path(info_id),
    }
  }

  pub fn info_mut_from_path(&mut self, info_id: &TestInfoId) -> Option<&mut TestInfo> {
    let case = self.cases.iter_mut().find(|case| case.info.info_id == *info_id)?;
    match info_id {
      TestInfoId::Suite(_) => None,
      TestInfoId::CaseMulti(_, _) | TestInfoId::CaseStep(_, _) => Some(&mut case.info),
      TestInfoId::Step(_, _, _) => case.info_mut_from_path(info_id),
    }
  }
}
