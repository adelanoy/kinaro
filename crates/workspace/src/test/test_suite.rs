use crate::error::{ProjectError::TestNotFound, ProjectResult};
use crate::test::{TestInfo, TestInfoId, test_case::TestCase};
use gpui_kit::SharedString;
use ki_project::FileTestSuite;
use log::{error, warn};
use std::fmt::Debug;
use uuid::Uuid;

/// A test suite: the root of the test tree.
///
/// A suite owns its metadata ([`TestInfo`]) and a flat list of [`TestCase`]s.
/// Each case is either a *multi-step case* (a case that holds its own steps) or
/// a *case step* (a single step promoted directly to the suite level), so the
/// tree is addressed through [`TestInfoId`] paths rather than by index.
///
/// Suites are created with [`TestSuite::new`], or loaded from their on-disk
/// representation with [`TestSuite::from_file`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSuite {
  /// Metadata for the suite itself (id, name, description, disabled flag).
  ///
  /// `info.info_id` is always a [`TestInfoId::Suite`].
  pub info: TestInfo,
  /// The suite's direct children, in display order.
  pub cases: Vec<TestCase>,
}

impl TestSuite {
  pub fn add_test_case(&mut self, info_id: &TestInfoId)  {
    let position = self.cases.iter().position(|case| case.info.info_id.is_parent(info_id));
    let name = ki_utils::next_available_name("new Case", self.cases.iter().map(|case| case.info.name.clone()));
    match position {
      Some(ix) => self.cases.insert(ix + 1, TestCase::new(self.info.id(), name)),
      None => self.cases.push(TestCase::new(self.info.id(), name)),
    }
  }

  pub fn add_test_step(&mut self, info_id: &TestInfoId) -> ProjectResult<()> {
    match info_id {
      TestInfoId::Suite(_) | TestInfoId::CaseStep(_, _) => {
        let name = ki_utils::next_available_name("New Step", self.cases.iter().map(|case| case.info.name.clone()));
        self.cases.push(TestCase::new_case_step(self.info.id(), name));
        Ok(())
      }
      TestInfoId::CaseMulti(_, case_id) | TestInfoId::Step(_, case_id, _) => {
        let Some(case) = self.cases.iter_mut().find(|case| case.info.id() == *case_id) else {
          warn!("TestSuite:add_test_step: unknow path: {}", info_id);
          return Err(TestNotFound);
        };
        case.add_test_step(info_id)
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
    let case = self.cases.iter().find(|case| case.info.info_id.is_parent(info_id))?;
    match info_id {
      TestInfoId::Suite(_) => None,
      TestInfoId::CaseMulti(_, _) | TestInfoId::CaseStep(_, _) => Some(&case.info),
      TestInfoId::Step(_, _, _) => case.info_from_path(info_id),
    }
  }

  pub fn info_mut_from_path(&mut self, info_id: &TestInfoId) -> Option<&mut TestInfo> {
    let case = self.cases.iter_mut().find(|case| case.info.info_id.is_parent(info_id))?;
    match info_id {
      TestInfoId::Suite(_) => None,
      TestInfoId::CaseMulti(_, _) | TestInfoId::CaseStep(_, _) => Some(&mut case.info),
      TestInfoId::Step(_, _, _) => case.info_mut_from_path(info_id),
    }
  }

  pub fn new(name: SharedString) -> Self {
    Self {
      info: TestInfo {
        info_id: TestInfoId::Suite(Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      cases: vec![],
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::test::TestInfoId;
  use crate::test::test_suite::TestSuite;
  use gpui_kit::SharedString;
  use ki_project::{FileTestCase, FileTestCaseType, FileTestInfo, FileTestStep, FileTestSuite};
  use uuid::Uuid;

  /// A suite holding one multi-step case (with two steps) and one case step,
  /// built through [`TestSuite::from_file`] so that every id is known upfront.
  struct Fixture {
    suite: TestSuite,
    suite_id: Uuid,
    multi_id: Uuid,
    step_a_id: Uuid,
    step_b_id: Uuid,
    case_step_id: Uuid,
  }

  fn file_info(id: Uuid, name: &str) -> FileTestInfo {
    FileTestInfo {
      id,
      name: name.to_string(),
      description: vec![],
      disabled: false,
    }
  }

  fn fixture() -> Fixture {
    let suite_id = Uuid::new_v4();
    let multi_id = Uuid::new_v4();
    let step_a_id = Uuid::new_v4();
    let step_b_id = Uuid::new_v4();
    let case_step_id = Uuid::new_v4();

    let suite = TestSuite::from_file(FileTestSuite {
      info: file_info(suite_id, "suite"),
      cases: vec![
        FileTestCase {
          info: file_info(multi_id, "multi case"),
          case_type: FileTestCaseType::CaseMulti {
            steps: vec![
              FileTestStep {
                info: file_info(step_a_id, "step a"),
                data: String::new(),
              },
              FileTestStep {
                info: file_info(step_b_id, "step b"),
                data: String::new(),
              },
            ],
          },
        },
        FileTestCase {
          info: file_info(case_step_id, "case step"),
          case_type: FileTestCaseType::CaseStep { data: String::new() },
        },
      ],
    });

    Fixture {
      suite,
      suite_id,
      multi_id,
      step_a_id,
      step_b_id,
      case_step_id,
    }
  }

  #[test]
  fn info_from_path_suite() {
    let f = fixture();
    // The suite's own info is not reachable through the path lookup.
    assert!(f.suite.info_from_path(&TestInfoId::Suite(f.suite_id)).is_none());
    assert!(f.suite.info_from_path(&TestInfoId::Suite(Uuid::new_v4())).is_none());
  }

  #[test]
  fn info_from_path_case_multi() {
    let f = fixture();
    let info = f
      .suite
      .info_from_path(&TestInfoId::CaseMulti(f.suite_id, f.multi_id))
      .expect("multi case should be reachable");
    assert_eq!(info.id(), f.multi_id);
    assert_eq!(info.name, SharedString::new("multi case"));
    assert!(matches!(info.info_id(), TestInfoId::CaseMulti(suite, case) if suite == f.suite_id && case == f.multi_id));
  }

  #[test]
  fn info_from_path_case_multi_unknown() {
    let f = fixture();
    // Unknown case id, right suite.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::CaseMulti(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    // Known case id, wrong suite.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::CaseMulti(Uuid::new_v4(), f.multi_id))
        .is_none()
    );
    // Right ids, but the addressed child is a case step, not a multi-step case.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::CaseMulti(f.suite_id, f.case_step_id))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_case_step() {
    let f = fixture();
    let info = f
      .suite
      .info_from_path(&TestInfoId::CaseStep(f.suite_id, f.case_step_id))
      .expect("case step should be reachable");
    assert_eq!(info.id(), f.case_step_id);
    assert_eq!(info.name, SharedString::new("case step"));
    assert!(
      matches!(info.info_id(), TestInfoId::CaseStep(suite, case) if suite == f.suite_id && case == f.case_step_id)
    );
  }

  #[test]
  fn info_from_path_case_step_unknown() {
    let f = fixture();
    // Unknown case id, right suite.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::CaseStep(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    // Known case id, wrong suite.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::CaseStep(Uuid::new_v4(), f.case_step_id))
        .is_none()
    );
    // Right ids, but the addressed child is a multi-step case, not a case step.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::CaseStep(f.suite_id, f.multi_id))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_step() {
    let f = fixture();
    let info = f
      .suite
      .info_from_path(&TestInfoId::Step(f.suite_id, f.multi_id, f.step_a_id))
      .expect("first step should be reachable");
    assert_eq!(info.id(), f.step_a_id);
    assert_eq!(info.name, SharedString::new("step a"));

    let info = f
      .suite
      .info_from_path(&TestInfoId::Step(f.suite_id, f.multi_id, f.step_b_id))
      .expect("second step should be reachable");
    assert_eq!(info.id(), f.step_b_id);
    assert_eq!(info.name, SharedString::new("step b"));
    assert!(
      matches!(info.info_id(), TestInfoId::Step(suite, case, step) if suite == f.suite_id && case == f.multi_id && step == f.step_b_id)
    );
  }

  #[test]
  fn info_from_path_step_unknown() {
    let f = fixture();
    // Owning case found, unknown step id.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::Step(f.suite_id, f.multi_id, Uuid::new_v4()))
        .is_none()
    );
    // Known step id, unknown case id.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::Step(f.suite_id, Uuid::new_v4(), f.step_a_id))
        .is_none()
    );
    // Known step id, wrong suite.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::Step(Uuid::new_v4(), f.multi_id, f.step_a_id))
        .is_none()
    );
    // A case step has no children.
    assert!(
      f.suite
        .info_from_path(&TestInfoId::Step(f.suite_id, f.case_step_id, Uuid::new_v4()))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_empty_suite() {
    let suite = TestSuite::new(SharedString::new("suite"));
    let suite_id = suite.info.id();
    assert!(suite.info_from_path(&TestInfoId::Suite(suite_id)).is_none());
    assert!(
      suite
        .info_from_path(&TestInfoId::CaseMulti(suite_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      suite
        .info_from_path(&TestInfoId::CaseStep(suite_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      suite
        .info_from_path(&TestInfoId::Step(suite_id, Uuid::new_v4(), Uuid::new_v4()))
        .is_none()
    );
  }

  #[test]
  fn info_mut_from_path_suite() {
    let mut f = fixture();
    assert!(f.suite.info_mut_from_path(&TestInfoId::Suite(f.suite_id)).is_none());
    assert!(f.suite.info_mut_from_path(&TestInfoId::Suite(Uuid::new_v4())).is_none());
  }

  #[test]
  fn info_mut_from_path_case_multi() {
    let mut f = fixture();
    let info_id = TestInfoId::CaseMulti(f.suite_id, f.multi_id);
    let info = f.suite.info_mut_from_path(&info_id).expect("multi case should be reachable");
    assert_eq!(info.id(), f.multi_id);
    info.name = SharedString::new("renamed case");
    info.disabled = true;

    let info = f.suite.info_from_path(&info_id).expect("multi case should be reachable");
    assert_eq!(info.name, SharedString::new("renamed case"));
    assert!(info.disabled);
  }

  #[test]
  fn info_mut_from_path_case_multi_unknown() {
    let mut f = fixture();
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::CaseMulti(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::CaseMulti(Uuid::new_v4(), f.multi_id))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::CaseMulti(f.suite_id, f.case_step_id))
        .is_none()
    );
  }

  #[test]
  fn info_mut_from_path_case_step() {
    let mut f = fixture();
    let info_id = TestInfoId::CaseStep(f.suite_id, f.case_step_id);
    let info = f.suite.info_mut_from_path(&info_id).expect("case step should be reachable");
    assert_eq!(info.id(), f.case_step_id);
    info.name = SharedString::new("renamed case step");

    let info = f.suite.info_from_path(&info_id).expect("case step should be reachable");
    assert_eq!(info.name, SharedString::new("renamed case step"));
  }

  #[test]
  fn info_mut_from_path_case_step_unknown() {
    let mut f = fixture();
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::CaseStep(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::CaseStep(Uuid::new_v4(), f.case_step_id))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::CaseStep(f.suite_id, f.multi_id))
        .is_none()
    );
  }

  #[test]
  fn info_mut_from_path_step() {
    let mut f = fixture();
    let info_id = TestInfoId::Step(f.suite_id, f.multi_id, f.step_b_id);
    let info = f.suite.info_mut_from_path(&info_id).expect("second step should be reachable");
    assert_eq!(info.id(), f.step_b_id);
    info.name = SharedString::new("renamed step");
    info.disabled = true;

    let info = f.suite.info_from_path(&info_id).expect("second step should be reachable");
    assert_eq!(info.name, SharedString::new("renamed step"));
    assert!(info.disabled);
    // The sibling step is untouched.
    let sibling = f
      .suite
      .info_from_path(&TestInfoId::Step(f.suite_id, f.multi_id, f.step_a_id))
      .expect("first step should be reachable");
    assert_eq!(sibling.name, SharedString::new("step a"));
    assert!(!sibling.disabled);
  }

  #[test]
  fn info_mut_from_path_step_unknown() {
    let mut f = fixture();
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::Step(f.suite_id, f.multi_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::Step(f.suite_id, Uuid::new_v4(), f.step_a_id))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::Step(Uuid::new_v4(), f.multi_id, f.step_a_id))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestInfoId::Step(f.suite_id, f.case_step_id, Uuid::new_v4()))
        .is_none()
    );
  }

  fn case_names(suite: &TestSuite) -> Vec<String> {
    suite.cases.iter().map(|case| case.info.name.to_string()).collect()
  }

  #[test]
  fn add_test_case_suite_selected_appends_at_end() {
    let mut f = fixture();
    f.suite
      .add_test_case(&TestInfoId::Suite(f.suite_id));
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step", "new Case"]);
  }

  #[test]
  fn add_test_case_after_case_multi() {
    let mut f = fixture();
    f.suite
      .add_test_case(&TestInfoId::Step(f.suite_id, f.multi_id, f.step_a_id));
    assert_eq!(case_names(&f.suite), vec!["multi case", "new Case", "case step"]);
  }

  #[test]
  fn add_test_case_after_case_step() {
    let mut f = fixture();
    f.suite
      .add_test_case(&TestInfoId::CaseStep(f.suite_id, f.case_step_id));
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step", "new Case"]);
  }

  #[test]
  fn add_test_case_unknown_path_appends_at_end() {
    let mut f = fixture();
    // A path for a different suite entirely: no case in this suite is its parent.
    f.suite
      .add_test_case(&TestInfoId::CaseMulti(Uuid::new_v4(), Uuid::new_v4()));
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step", "new Case"]);
  }

  #[test]
  fn add_test_case_name_is_made_unique() {
    let mut f = fixture();
    f.suite
      .add_test_case(&TestInfoId::Suite(f.suite_id));
    f.suite
      .add_test_case(&TestInfoId::Suite(f.suite_id));
    assert_eq!(
      case_names(&f.suite),
      vec!["multi case", "case step", "new Case", "new Case_1"]
    );
  }
}
