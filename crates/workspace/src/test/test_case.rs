use crate::error::ProjectError::TestNotFound;
use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestInfo, TestInfoId, test_step::TestStep};
use gpui_kit::SharedString;
use ki_project::{FileTestCase, FileTestCaseType};
use log::error;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestCaseType {
  CaseMulti { steps: Vec<TestStep> },
  CaseStep { data: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
  pub info: TestInfo,
  case_type: TestCaseType,
}

impl TestCase {
  pub fn add_test_step(&mut self, info_id: &TestInfoId) -> ProjectResult<()> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        let position = steps.iter().position(|step| step.info.info_id == *info_id);
        let name = ki_utils::next_available_name("New Step", steps.iter().map(|step| step.info.name.clone()));
        match position {
          Some(ix) => steps.insert(ix + 1, TestStep::new(info_id.suite_id(), self.info.id(), name)),
          None => steps.push(TestStep::new(info_id.suite_id(), self.info.id(), name)),
        }
        Ok(())
      }
      TestCaseType::CaseStep { .. } => {
        error!(
          "TestCase:add_test_step: cannot add a step on step case at: {}",
          self.info.info_id
        );
        Err(ProjectError::OperationNotAllowed)
      }
    }
  }

  #[inline]
  pub fn case_type(&self) -> &TestCaseType {
    &self.case_type
  }

  /// Duplicates this TestCase with a new random ID and the given name. All children are also duplicated with a new ID, but keep their
  /// original name, whether this is a regular or a step case
  pub(super) fn duplicate(&self, name: SharedString) -> Self {
    Self {
      info: self.info.duplicate(name),
      case_type: match &self.case_type {
        TestCaseType::CaseMulti { steps } => TestCaseType::CaseMulti {
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
      TestCaseType::CaseMulti { steps } => {
        let Some(ix) = steps.iter().position(|step| step.info.info_id == *info_id) else {
          error!("TestCase:duplicate_child: unknow path: {}", info_id);
          return Err(TestNotFound);
        };
        let name = ki_utils::next_available_name(&steps[ix].info.name, steps.iter().map(|case| case.info.name.clone()));
        let duplicate = steps[ix].duplicate(name);
        steps.insert(ix + 1, duplicate);
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

  pub(crate) fn from_file(file_test_case: FileTestCase, suite_id: Uuid) -> Self {
    let id = file_test_case.info.id;
    match file_test_case.case_type {
      FileTestCaseType::CaseMulti { steps } => Self {
        info: TestInfo::new_case(file_test_case.info, suite_id),
        case_type: TestCaseType::CaseMulti {
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
        TestCaseType::CaseMulti { steps } => FileTestCaseType::CaseMulti {
          steps: steps.iter().map(|step| step.get_file()).collect(),
        },
        TestCaseType::CaseStep { data } => FileTestCaseType::CaseStep { data: data.clone() },
      },
    }
  }

  #[allow(unused)]
  pub fn info_from_path(&self, info_id: &TestInfoId) -> Option<&TestInfo> {
    match &self.case_type {
      TestCaseType::CaseMulti { steps } => steps.iter().find(|step| step.info.info_id == *info_id).map(|step| &step.info),
      TestCaseType::CaseStep { .. } => None,
    }
  }

  pub fn info_mut_from_path(&mut self, info_id: &TestInfoId) -> Option<&mut TestInfo> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => steps
        .iter_mut()
        .find(|step| step.info.info_id == *info_id)
        .map(|step| &mut step.info),
      TestCaseType::CaseStep { .. } => None,
    }
  }

  #[inline]
  pub fn is_step(&self) -> bool {
    matches!(self.case_type, TestCaseType::CaseStep { .. })
  }

  pub fn new(suite_id: Uuid, name: SharedString) -> Self {
    Self {
      info: TestInfo {
        info_id: TestInfoId::CaseMulti(suite_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      case_type: TestCaseType::CaseMulti { steps: vec![] },
    }
  }

  pub fn new_case_step(suite_id: Uuid, name: SharedString) -> Self {
    Self {
      info: TestInfo {
        info_id: TestInfoId::CaseStep(suite_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      case_type: TestCaseType::CaseStep { data: "".to_string() },
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::error::ProjectError;
  use crate::test::TestInfoId;
  use crate::test::test_case::{TestCase, TestCaseType};
  use gpui_kit::SharedString;
  use ki_project::{FileTestCase, FileTestCaseType, FileTestInfo, FileTestStep};
  use uuid::Uuid;

  /// A multi-step case holding two steps, built through [`TestCase::from_file`]
  /// so that every id is known upfront.
  struct Fixture {
    case: TestCase,
    suite_id: Uuid,
    case_id: Uuid,
    step_a_id: Uuid,
    step_b_id: Uuid,
  }

  fn file_info(id: Uuid, name: &str) -> FileTestInfo {
    FileTestInfo {
      id,
      name: name.to_string(),
      description: None,
      disabled: false,
    }
  }

  fn fixture() -> Fixture {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_a_id = Uuid::new_v4();
    let step_b_id = Uuid::new_v4();

    let case = TestCase::from_file(
      FileTestCase {
        info: file_info(case_id, "multi case"),
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
      suite_id,
    );

    Fixture {
      case,
      suite_id,
      case_id,
      step_a_id,
      step_b_id,
    }
  }

  /// A case step, built through [`TestCase::from_file`].
  struct CaseStepFixture {
    case: TestCase,
    suite_id: Uuid,
    case_id: Uuid,
  }

  fn case_step_fixture() -> CaseStepFixture {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();

    let case = TestCase::from_file(
      FileTestCase {
        info: file_info(case_id, "case step"),
        case_type: FileTestCaseType::CaseStep {
          data: "payload".to_string(),
        },
      },
      suite_id,
    );

    CaseStepFixture { case, suite_id, case_id }
  }

  fn step_names(case: &TestCase) -> Vec<String> {
    match case.case_type() {
      TestCaseType::CaseMulti { steps } => steps.iter().map(|step| step.info.name.to_string()).collect(),
      TestCaseType::CaseStep { .. } => vec![],
    }
  }

  #[test]
  fn add_test_step_on_empty_case_multi() {
    let suite_id = Uuid::new_v4();
    let mut case = TestCase::new(suite_id, SharedString::new("case"));
    let case_id = case.info.id();
    case
      .add_test_step(&TestInfoId::CaseMulti(suite_id, case_id))
      .expect("add_test_step should succeed");
    assert_eq!(step_names(&case), vec!["New Step"]);
  }

  #[test]
  fn add_test_step_after_known_step() {
    let mut f = fixture();
    f.case
      .add_test_step(&TestInfoId::Step(f.suite_id, f.case_id, f.step_a_id))
      .expect("add_test_step should succeed");
    assert_eq!(step_names(&f.case), vec!["step a", "New Step", "step b"]);
  }

  #[test]
  fn add_test_step_name_is_made_unique() {
    let mut f = fixture();
    // A CaseMulti path never matches a step's own info_id, so both calls fall
    // through to appending at the end.
    f.case.add_test_step(&TestInfoId::CaseMulti(f.suite_id, f.case_id)).unwrap();
    f.case.add_test_step(&TestInfoId::CaseMulti(f.suite_id, f.case_id)).unwrap();
    assert_eq!(step_names(&f.case), vec!["step a", "step b", "New Step", "New Step_1"]);
  }

  #[test]
  fn add_test_step_on_case_step_is_not_allowed() {
    let mut f = case_step_fixture();
    let err = f
      .case
      .add_test_step(&TestInfoId::CaseStep(f.suite_id, f.case_id))
      .expect_err("adding a step to a case step should be rejected");
    assert!(matches!(err, ProjectError::OperationNotAllowed));
  }

  #[test]
  fn case_type_reflects_variant() {
    let f = fixture();
    assert!(matches!(f.case.case_type(), TestCaseType::CaseMulti { .. }));
    let step_f = case_step_fixture();
    assert!(matches!(step_f.case.case_type(), TestCaseType::CaseStep { .. }));
  }

  #[test]
  fn is_step_true_for_case_step_false_for_case_multi() {
    let f = fixture();
    assert!(!f.case.is_step());
    let step_f = case_step_fixture();
    assert!(step_f.case.is_step());
  }

  #[test]
  fn duplicate_case_multi_generates_fresh_ids_and_keeps_step_names() {
    let f = fixture();
    let dup = f.case.duplicate(SharedString::new("multi case copy"));

    assert_eq!(dup.info.name, SharedString::new("multi case copy"));
    assert_ne!(dup.info.id(), f.case_id);
    assert!(matches!(dup.info.info_id(), TestInfoId::CaseMulti(suite, case) if suite == f.suite_id && case != f.case_id));
    assert_eq!(step_names(&dup), vec!["step a", "step b"]);

    match dup.case_type() {
      TestCaseType::CaseMulti { steps } => {
        assert_ne!(steps[0].info.id(), f.step_a_id);
        assert_ne!(steps[1].info.id(), f.step_b_id);
      }
      TestCaseType::CaseStep { .. } => panic!("expected a CaseMulti"),
    }
  }

  #[test]
  fn duplicate_case_step_keeps_data() {
    let f = case_step_fixture();
    let dup = f.case.duplicate(SharedString::new("case step copy"));

    assert_eq!(dup.info.name, SharedString::new("case step copy"));
    assert_ne!(dup.info.id(), f.case_id);
    match dup.case_type() {
      TestCaseType::CaseStep { data } => assert_eq!(data, "payload"),
      TestCaseType::CaseMulti { .. } => panic!("expected a CaseStep"),
    }
  }

  #[test]
  fn duplicate_child_inserts_after_step() {
    let mut f = fixture();
    f.case
      .duplicate_child(&TestInfoId::Step(f.suite_id, f.case_id, f.step_a_id))
      .expect("duplicate_child should succeed");
    // The copy's name is made unique against its siblings.
    assert_eq!(step_names(&f.case), vec!["step a", "step a_1", "step b"]);
  }

  #[test]
  fn duplicate_child_of_last_step_appends_at_end() {
    let mut f = fixture();
    f.case
      .duplicate_child(&TestInfoId::Step(f.suite_id, f.case_id, f.step_b_id))
      .expect("duplicate_child should succeed");
    assert_eq!(step_names(&f.case), vec!["step a", "step b", "step b_1"]);
  }

  #[test]
  fn duplicate_child_unknown_step_is_not_found() {
    let mut f = fixture();
    let err = f
      .case
      .duplicate_child(&TestInfoId::Step(f.suite_id, f.case_id, Uuid::new_v4()))
      .expect_err("an unknown step id should fail");
    assert!(matches!(err, ProjectError::TestNotFound));
  }

  #[test]
  fn duplicate_child_on_case_step_is_not_allowed() {
    let mut f = case_step_fixture();
    let err = f
      .case
      .duplicate_child(&TestInfoId::CaseStep(f.suite_id, f.case_id))
      .expect_err("duplicating children of a case step should be rejected");
    assert!(matches!(err, ProjectError::OperationNotAllowed));
  }

  #[test]
  fn from_file_builds_case_multi_with_step_ids() {
    let f = fixture();
    assert_eq!(f.case.info.name, SharedString::new("multi case"));
    assert!(matches!(f.case.info.info_id(), TestInfoId::CaseMulti(suite, case) if suite == f.suite_id && case == f.case_id));
    assert_eq!(step_names(&f.case), vec!["step a", "step b"]);
  }

  #[test]
  fn get_file_round_trips_case_multi() {
    let f = fixture();
    let rebuilt = TestCase::from_file(f.case.get_file(), f.suite_id);
    assert_eq!(rebuilt, f.case);
  }

  #[test]
  fn get_file_round_trips_case_step() {
    let f = case_step_fixture();
    let rebuilt = TestCase::from_file(f.case.get_file(), f.suite_id);
    assert_eq!(rebuilt, f.case);
  }

  #[test]
  fn info_from_path_step_known() {
    let f = fixture();
    let info = f
      .case
      .info_from_path(&TestInfoId::Step(f.suite_id, f.case_id, f.step_b_id))
      .expect("step b should be reachable");
    assert_eq!(info.id(), f.step_b_id);
    assert_eq!(info.name, SharedString::new("step b"));
  }

  #[test]
  fn info_from_path_step_unknown() {
    let f = fixture();
    assert!(
      f.case
        .info_from_path(&TestInfoId::Step(f.suite_id, f.case_id, Uuid::new_v4()))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_on_case_step_is_always_none() {
    let f = case_step_fixture();
    assert!(f.case.info_from_path(&TestInfoId::CaseStep(f.suite_id, f.case_id)).is_none());
  }

  #[test]
  fn info_mut_from_path_step_mutates_in_place() {
    let mut f = fixture();
    let info_id = TestInfoId::Step(f.suite_id, f.case_id, f.step_a_id);
    let info = f.case.info_mut_from_path(&info_id).expect("step a should be reachable");
    info.name = SharedString::new("renamed step");
    info.disabled = true;

    let info = f.case.info_from_path(&info_id).expect("step a should be reachable");
    assert_eq!(info.name, SharedString::new("renamed step"));
    assert!(info.disabled);

    // The sibling step is untouched.
    let sibling = f
      .case
      .info_from_path(&TestInfoId::Step(f.suite_id, f.case_id, f.step_b_id))
      .expect("step b should be reachable");
    assert_eq!(sibling.name, SharedString::new("step b"));
    assert!(!sibling.disabled);
  }

  #[test]
  fn info_mut_from_path_on_case_step_is_always_none() {
    let mut f = case_step_fixture();
    assert!(
      f.case
        .info_mut_from_path(&TestInfoId::CaseStep(f.suite_id, f.case_id))
        .is_none()
    );
  }

  #[test]
  fn new_creates_empty_case_multi() {
    let suite_id = Uuid::new_v4();
    let case = TestCase::new(suite_id, SharedString::new("new case"));
    assert_eq!(case.info.name, SharedString::new("new case"));
    assert!(matches!(case.info.info_id(), TestInfoId::CaseMulti(suite, _) if suite == suite_id));
    assert!(matches!(case.case_type(), TestCaseType::CaseMulti { steps } if steps.is_empty()));
    assert!(!case.is_step());
  }

  #[test]
  fn new_case_step_creates_empty_data() {
    let suite_id = Uuid::new_v4();
    let case = TestCase::new_case_step(suite_id, SharedString::new("new case step"));
    assert_eq!(case.info.name, SharedString::new("new case step"));
    assert!(matches!(case.info.info_id(), TestInfoId::CaseStep(suite, _) if suite == suite_id));
    assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if data.is_empty()));
    assert!(case.is_step());
  }
}
