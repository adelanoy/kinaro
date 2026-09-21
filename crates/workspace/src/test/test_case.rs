use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestMetadata, TestPath, test_step::TestStep};
use gpui_kit::SharedString;
use ki_project::{FileTestCase, FileTestCaseType};
use log::error;
use uuid::Uuid;

/// The two shapes a [`TestCase`] can take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestCaseType {
  /// A case that owns its own list of steps.
  CaseMulti { steps: Vec<TestStep> },
  /// A case level step, so that it can be positioned directly under a suite
  CaseStep { data: String },
}

/// A test case: either a [`TestCaseType::CaseMulti`] holding its own steps,
/// or a [`TestCaseType::CaseStep`] (a case level step).
///
/// Cases are created with [`TestCase::new_multi`] / [`TestCase::new_step`], or
/// loaded from their on-disk representation with [`TestCase::from_file`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
  /// Metadata for the case itself (id, name, description, disabled flag).
  pub meta: TestMetadata,
  case_type: TestCaseType,
}

impl TestCase {
  /// Adds a new step relative to `path`: right after the step whose id
  /// matches, or appended at the end when nothing matches. Only valid on a
  /// [`TestCaseType::CaseMulti`].
  ///
  /// # Errors
  /// Returns [`ProjectError::OperationNotAllowed`] when called on a
  /// [`TestCaseType::CaseStep`], since a case step cannot have children.
  pub fn add_test_step(&mut self, path: TestPath) -> ProjectResult<TestPath> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        let position = steps.iter().position(|step| step.meta.path == path);
        let name = ki_utils::next_available_name("New Step", steps.iter().map(|step| step.meta.name.clone()));
        let path = match position {
          Some(ix) => {
            let new_step = TestStep::new(path.suite_id(), self.meta.id(), name);
            let path = new_step.meta.path;
            steps.insert(ix + 1, new_step);
            path
          }
          None => {
            let new_step = TestStep::new(path.suite_id(), self.meta.id(), name);
            let path = new_step.meta.path;
            steps.push(new_step);
            path
          }
        };
        Ok(path)
      }
      TestCaseType::CaseStep { .. } => {
        error!(
          "TestCase:add_test_step: cannot add a step on step case at: {}",
          self.meta.path
        );
        Err(ProjectError::OperationNotAllowed)
      }
    }
  }

  /// Returns the [`TestCaseType`] variant backing this case.
  #[inline]
  pub fn case_type(&self) -> &TestCaseType {
    &self.case_type
  }

  /// Returns the mutable [`TestCaseType`] variant backing this case.
  #[inline]
  pub fn case_type_mut(&mut self) -> &mut TestCaseType {
    &mut self.case_type
  }

  /// Deletes the step addressed by `path` from this case. Only valid on a
  /// [`TestCaseType::CaseMulti`]; a [`TestCaseType::CaseStep`] has no
  /// children to delete.
  ///
  /// # Errors
  /// - [`ProjectError::TestNotFound`] if no step matches `path`.
  /// - [`ProjectError::OperationNotAllowed`] when called on a
  ///   [`TestCaseType::CaseStep`].
  pub(crate) fn delete_test(&mut self, path: &TestPath) -> ProjectResult<()> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        let Some(ix) = steps.iter().position(|step| step.meta.path.is_parent(path)) else {
          error!("TestCase:delete_test: unknow path: {}", path);
          return Err(ProjectError::TestNotFound(*path));
        };
        steps.remove(ix);
        Ok(())
      }
      TestCaseType::CaseStep { .. } => {
        error!(
          "TestCase:delete_test: cannot delete a step on step case at: {}",
          self.meta.path
        );
        Err(ProjectError::OperationNotAllowed)
      }
    }
  }

  /// Duplicates this case with a new random id and the given name. All
  /// children in a `CaseMulti` are also duplicated with a new id,
  /// but keep their original name.
  pub(super) fn duplicate(&self, name: SharedString) -> Self {
    Self {
      meta: self.meta.duplicate(name),
      case_type: match &self.case_type {
        TestCaseType::CaseMulti { steps } => TestCaseType::CaseMulti {
          steps: steps.iter().map(|step| step.duplicate(step.meta.name.clone())).collect(),
        },
        TestCaseType::CaseStep { data } => TestCaseType::CaseStep { data: data.clone() },
      },
    }
  }

  /// Duplicates the step addressed by `path`, inserting the copy right
  /// after it with a name made unique against its siblings.
  ///
  /// # Errors
  /// - [`ProjectError::TestNotFound`] if no step matches `path`.
  /// - [`ProjectError::OperationNotAllowed`] when called on a
  ///   [`TestCaseType::CaseStep`], since it has no children.
  pub(crate) fn duplicate_child(&mut self, path: &TestPath) -> ProjectResult<()> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        let Some(ix) = steps.iter().position(|step| step.meta.path == *path) else {
          error!("TestCase:duplicate_child: unknow path: {}", path);
          return Err(ProjectError::TestNotFound(*path));
        };
        let name = ki_utils::next_available_name(&steps[ix].meta.name, steps.iter().map(|case| case.meta.name.clone()));
        let duplicate = steps[ix].duplicate(name);
        steps.insert(ix + 1, duplicate);
        Ok(())
      }
      TestCaseType::CaseStep { .. } => {
        error!(
          "TestCase:duplicate_child: cannot duplicated children on a step case. path: {}",
          path
        );
        Err(ProjectError::OperationNotAllowed)
      }
    }
  }

  /// Builds a case from its on-disk representation.
  pub(crate) fn from_file(file_test_case: FileTestCase, suite_id: Uuid) -> Self {
    let id = file_test_case.info.id;
    let info = TestMetadata::new_case(file_test_case.info, suite_id);
    match file_test_case.case_type {
      FileTestCaseType::CaseMulti { steps } => Self {
        meta: info,
        case_type: TestCaseType::CaseMulti {
          steps: steps
            .into_iter()
            .map(|step| TestStep::from_file(step, suite_id, id))
            .collect(),
        },
      },
      FileTestCaseType::CaseStep { data } => Self {
        meta: info,
        case_type: TestCaseType::CaseStep { data },
      },
    }
  }

  /// Promotes a step to a case-level [`TestCaseType::CaseStep`], keeping
  /// the step's own id as the new case's id and reparenting it under
  /// `suite_id`.
  pub(crate) fn from_step(step: TestStep, suite_id: Uuid) -> Self {
    let TestStep { mut meta, data } = step;
    let id = meta.id();
    meta.path = TestPath::Case(suite_id, id);
    Self {
      meta,
      case_type: TestCaseType::CaseStep { data },
    }
  }

  /// Produces the serializable, on-disk representation of this case.
  pub(crate) fn get_file(&self) -> FileTestCase {
    FileTestCase {
      info: self.meta.to_file(),
      case_type: match &self.case_type {
        TestCaseType::CaseMulti { steps } => FileTestCaseType::CaseMulti {
          steps: steps.iter().map(|step| step.get_file()).collect(),
        },
        TestCaseType::CaseStep { data } => FileTestCaseType::CaseStep { data: data.clone() },
      },
    }
  }

  /// Looks up the [`TestMetadata`] of the step addressed by `path`. Always
  /// `None` on a [`TestCaseType::CaseStep`], since it has no children.
  #[allow(unused)]
  pub fn info_from_path(&self, path: &TestPath) -> Option<&TestMetadata> {
    match &self.case_type {
      TestCaseType::CaseMulti { steps } => steps.iter().find(|step| step.meta.path == *path).map(|step| &step.meta),
      TestCaseType::CaseStep { .. } => None,
    }
  }

  /// Mutable counterpart to [`TestCase::info_from_path`].
  pub fn info_mut_from_path(&mut self, path: &TestPath) -> Option<&mut TestMetadata> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => steps
        .iter_mut()
        .find(|step| step.meta.path == *path)
        .map(|step| &mut step.meta),
      TestCaseType::CaseStep { .. } => None,
    }
  }

  /// Returns `true` if this case is a [`TestCaseType::CaseStep`].
  #[inline]
  pub fn is_case_step(&self) -> bool {
    matches!(self.case_type, TestCaseType::CaseStep { .. })
  }

  /// Creates a new, empty [`TestCaseType::CaseMulti`] with a freshly
  /// generated id.
  pub fn new_multi(suite_id: Uuid, name: SharedString) -> Self {
    Self {
      meta: TestMetadata {
        path: TestPath::Case(suite_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      case_type: TestCaseType::CaseMulti { steps: vec![] },
    }
  }

  /// Creates a new [`TestCaseType::CaseStep`] with empty data and a freshly
  /// generated id.
  pub fn new_step(suite_id: Uuid, name: SharedString) -> Self {
    Self {
      meta: TestMetadata {
        path: TestPath::Case(suite_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      case_type: TestCaseType::CaseStep { data: "".to_string() },
    }
  }

  /// Removes and returns the step matching `step_id` from this case's
  /// steps, unmodified (its `meta.path` still addresses its old case).
  /// Returns `None` when this is a [`TestCaseType::CaseStep`], or no step
  /// matches.
  pub(crate) fn remove_step(&mut self, step_id: Uuid) -> Option<TestStep> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        let ix = steps.iter().position(|step| step.meta.id() == step_id)?;
        Some(steps.remove(ix))
      }
      TestCaseType::CaseStep { .. } => None,
    }
  }

  /// Moves this case under a different suite, keeping its own id. Also
  /// reparents a [`TestCaseType::CaseMulti`]'s steps under the new suite
  /// (same case id, their own step ids kept). A no-op when `suite_id` is
  /// already its current suite.
  pub(crate) fn reparent(mut self, suite_id: Uuid) -> Self {
    if suite_id != self.meta.path.suite_id() {
      let case_id = self.meta.id();
      self.meta.path = TestPath::Case(suite_id, case_id);

      if let TestCaseType::CaseMulti { steps } = &mut self.case_type {
        steps.iter_mut().for_each(|step| {
          step.meta.path = TestPath::Step(suite_id, case_id, step.meta.id());
        });
      }
    }
    self
  }

  /// Converts this case to the step(s) it is equivalent to, at its own
  /// current suite/case location, without modifying this case. A
  /// [`TestCaseType::CaseMulti`] yields its steps, keeping their own ids; a
  /// [`TestCaseType::CaseStep`] yields a single step with a freshly
  /// generated id, carrying this case's name, description, disabled flag
  /// and data. To move the result elsewhere, reparent each returned
  /// [`TestStep`] via [`TestStep::reparent`](crate::test::test_step::TestStep::reparent).
  pub(crate) fn to_steps(&self) -> Vec<TestStep> {
    let suite_id = self.meta.path.suite_id();
    let case_id = self.meta.path.case_id().unwrap();
    match &self.case_type {
      TestCaseType::CaseMulti { steps } => steps
        .clone()
        .into_iter()
        .map(|step| {
          let TestStep { mut meta, data } = step;
          let id = meta.id();
          meta.path = TestPath::Step(suite_id, case_id, id);
          TestStep { meta, data }
        })
        .collect(),
      TestCaseType::CaseStep { data } => {
        let meta = TestMetadata {
          path: TestPath::Step(suite_id, case_id, Uuid::new_v4()),
          name: self.meta.name.clone(),
          description: self.meta.description.clone(),
          disabled: self.meta.disabled,
        };
        vec![TestStep {
          meta,
          data: data.clone(),
        }]
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::error::ProjectError;
  use crate::test::TestPath;
  use crate::test::test_case::{TestCase, TestCaseType};
  use crate::test::test_step::TestStep;
  use gpui_kit::SharedString;
  use ki_project::{FileTestCase, FileTestCaseType, FileTestMetadata, FileTestStep};
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

  fn file_info(id: Uuid, name: &str) -> FileTestMetadata {
    FileTestMetadata {
      id,
      name: name.to_string(),
      description: vec![],
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
      TestCaseType::CaseMulti { steps } => steps.iter().map(|step| step.meta.name.to_string()).collect(),
      TestCaseType::CaseStep { .. } => vec![],
    }
  }

  #[test]
  fn add_test_step_on_empty_case_multi() {
    let suite_id = Uuid::new_v4();
    let mut case = TestCase::new_multi(suite_id, SharedString::new("case"));
    let case_id = case.meta.id();
    case
      .add_test_step(TestPath::Case(suite_id, case_id))
      .expect("add_test_step should succeed");
    assert_eq!(step_names(&case), vec!["New Step"]);
  }

  #[test]
  fn add_test_step_after_known_step() {
    let mut f = fixture();
    f.case
      .add_test_step(TestPath::Step(f.suite_id, f.case_id, f.step_a_id))
      .expect("add_test_step should succeed");
    assert_eq!(step_names(&f.case), vec!["step a", "New Step", "step b"]);
  }

  #[test]
  fn add_test_step_name_is_made_unique() {
    let mut f = fixture();
    // A CaseMulti path never matches a step's own path, so both calls fall
    // through to appending at the end.
    f.case.add_test_step(TestPath::Case(f.suite_id, f.case_id)).unwrap();
    f.case.add_test_step(TestPath::Case(f.suite_id, f.case_id)).unwrap();
    assert_eq!(step_names(&f.case), vec!["step a", "step b", "New Step", "New Step_1"]);
  }

  #[test]
  fn add_test_step_on_case_step_is_not_allowed() {
    let mut f = case_step_fixture();
    let err = f
      .case
      .add_test_step(TestPath::Case(f.suite_id, f.case_id))
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
    assert!(!f.case.is_case_step());
    let step_f = case_step_fixture();
    assert!(step_f.case.is_case_step());
  }

  #[test]
  fn duplicate_case_multi_generates_fresh_ids_and_keeps_step_names() {
    let f = fixture();
    let dup = f.case.duplicate(SharedString::new("multi case copy"));

    assert_eq!(dup.meta.name, SharedString::new("multi case copy"));
    assert_ne!(dup.meta.id(), f.case_id);
    assert!(matches!(dup.meta.path(), TestPath::Case(suite, case) if suite == f.suite_id && case != f.case_id));
    assert_eq!(step_names(&dup), vec!["step a", "step b"]);

    match dup.case_type() {
      TestCaseType::CaseMulti { steps } => {
        assert_ne!(steps[0].meta.id(), f.step_a_id);
        assert_ne!(steps[1].meta.id(), f.step_b_id);
      }
      TestCaseType::CaseStep { .. } => panic!("expected a CaseMulti"),
    }
  }

  #[test]
  fn duplicate_case_step_keeps_data() {
    let f = case_step_fixture();
    let dup = f.case.duplicate(SharedString::new("case step copy"));

    assert_eq!(dup.meta.name, SharedString::new("case step copy"));
    assert_ne!(dup.meta.id(), f.case_id);
    match dup.case_type() {
      TestCaseType::CaseStep { data } => assert_eq!(data, "payload"),
      TestCaseType::CaseMulti { .. } => panic!("expected a CaseStep"),
    }
  }

  #[test]
  fn duplicate_child_inserts_after_step() {
    let mut f = fixture();
    f.case
      .duplicate_child(&TestPath::Step(f.suite_id, f.case_id, f.step_a_id))
      .expect("duplicate_child should succeed");
    // The copy's name is made unique against its siblings.
    assert_eq!(step_names(&f.case), vec!["step a", "step a_1", "step b"]);
  }

  #[test]
  fn duplicate_child_of_last_step_appends_at_end() {
    let mut f = fixture();
    f.case
      .duplicate_child(&TestPath::Step(f.suite_id, f.case_id, f.step_b_id))
      .expect("duplicate_child should succeed");
    assert_eq!(step_names(&f.case), vec!["step a", "step b", "step b_1"]);
  }

  #[test]
  fn duplicate_child_unknown_step_is_not_found() {
    let mut f = fixture();
    let err = f
      .case
      .duplicate_child(&TestPath::Step(f.suite_id, f.case_id, Uuid::new_v4()))
      .expect_err("an unknown step id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn duplicate_child_on_case_step_is_not_allowed() {
    let mut f = case_step_fixture();
    let err = f
      .case
      .duplicate_child(&TestPath::Case(f.suite_id, f.case_id))
      .expect_err("duplicating children of a case step should be rejected");
    assert!(matches!(err, ProjectError::OperationNotAllowed));
  }

  #[test]
  fn from_file_builds_case_multi_with_step_ids() {
    let f = fixture();
    assert_eq!(f.case.meta.name, SharedString::new("multi case"));
    assert!(matches!(f.case.meta.path(), TestPath::Case(suite, case) if suite == f.suite_id && case == f.case_id));
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
      .info_from_path(&TestPath::Step(f.suite_id, f.case_id, f.step_b_id))
      .expect("step b should be reachable");
    assert_eq!(info.id(), f.step_b_id);
    assert_eq!(info.name, SharedString::new("step b"));
  }

  #[test]
  fn info_from_path_step_unknown() {
    let f = fixture();
    assert!(
      f.case
        .info_from_path(&TestPath::Step(f.suite_id, f.case_id, Uuid::new_v4()))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_on_case_step_is_always_none() {
    let f = case_step_fixture();
    assert!(f.case.info_from_path(&TestPath::Case(f.suite_id, f.case_id)).is_none());
  }

  #[test]
  fn info_mut_from_path_step_mutates_in_place() {
    let mut f = fixture();
    let path = TestPath::Step(f.suite_id, f.case_id, f.step_a_id);
    let info = f.case.info_mut_from_path(&path).expect("step a should be reachable");
    info.name = SharedString::new("renamed step");
    info.disabled = true;

    let info = f.case.info_from_path(&path).expect("step a should be reachable");
    assert_eq!(info.name, SharedString::new("renamed step"));
    assert!(info.disabled);

    // The sibling step is untouched.
    let sibling = f
      .case
      .info_from_path(&TestPath::Step(f.suite_id, f.case_id, f.step_b_id))
      .expect("step b should be reachable");
    assert_eq!(sibling.name, SharedString::new("step b"));
    assert!(!sibling.disabled);
  }

  #[test]
  fn info_mut_from_path_on_case_step_is_always_none() {
    let mut f = case_step_fixture();
    assert!(f.case.info_mut_from_path(&TestPath::Case(f.suite_id, f.case_id)).is_none());
  }

  #[test]
  fn new_creates_empty_case_multi() {
    let suite_id = Uuid::new_v4();
    let case = TestCase::new_multi(suite_id, SharedString::new("new case"));
    assert_eq!(case.meta.name, SharedString::new("new case"));
    assert!(matches!(case.meta.path(), TestPath::Case(suite, _) if suite == suite_id));
    assert!(matches!(case.case_type(), TestCaseType::CaseMulti { steps } if steps.is_empty()));
    assert!(!case.is_case_step());
  }

  #[test]
  fn new_case_step_creates_empty_data() {
    let suite_id = Uuid::new_v4();
    let case = TestCase::new_step(suite_id, SharedString::new("new case step"));
    assert_eq!(case.meta.name, SharedString::new("new case step"));
    assert!(matches!(case.meta.path(), TestPath::Case(suite, _) if suite == suite_id));
    assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if data.is_empty()));
    assert!(case.is_case_step());
  }

  #[test]
  fn case_type_mut_allows_in_place_mutation() {
    let mut f = fixture();
    if let TestCaseType::CaseMulti { steps } = f.case.case_type_mut() {
      steps.clear();
    }
    assert!(step_names(&f.case).is_empty());
  }

  #[test]
  fn from_step_promotes_a_step_to_a_case_step_keeping_its_id() {
    let suite_id = Uuid::new_v4();
    let other_suite_id = Uuid::new_v4();
    let step = TestStep::new(suite_id, Uuid::new_v4(), SharedString::new("a step"));
    let step_id = step.meta.id();

    let case = TestCase::from_step(step, other_suite_id);

    assert!(case.is_case_step());
    assert_eq!(case.meta.id(), step_id);
    assert_eq!(case.meta.name, SharedString::new("a step"));
    assert!(matches!(case.meta.path(), TestPath::Case(suite, id) if suite == other_suite_id && id == step_id));
    assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if data.is_empty()));
  }

  #[test]
  fn remove_step_removes_the_matching_step() {
    let mut f = fixture();
    let removed = f.case.remove_step(f.step_a_id).expect("step a should be removable");
    assert_eq!(removed.meta.id(), f.step_a_id);
    assert_eq!(step_names(&f.case), vec!["step b"]);
  }

  #[test]
  fn remove_step_unknown_id_returns_none() {
    let mut f = fixture();
    assert!(f.case.remove_step(Uuid::new_v4()).is_none());
    assert_eq!(step_names(&f.case), vec!["step a", "step b"]);
  }

  #[test]
  fn remove_step_on_case_step_returns_none() {
    let mut f = case_step_fixture();
    assert!(f.case.remove_step(f.case_id).is_none());
  }

  #[test]
  fn reparent_updates_path_and_cascades_to_case_multi_steps() {
    let f = fixture();
    let new_suite = Uuid::new_v4();
    let case = f.case.reparent(new_suite);

    assert!(matches!(case.meta.path(), TestPath::Case(suite, id) if suite == new_suite && id == f.case_id));
    let TestCaseType::CaseMulti { steps } = case.case_type() else {
      panic!("expected a CaseMulti");
    };
    assert!(
      matches!(steps[0].meta.path(), TestPath::Step(suite, case, step) if suite == new_suite && case == f.case_id && step == f.step_a_id)
    );
    assert!(
      matches!(steps[1].meta.path(), TestPath::Step(suite, case, step) if suite == new_suite && case == f.case_id && step == f.step_b_id)
    );
  }

  #[test]
  fn reparent_is_a_no_op_for_the_same_suite() {
    let f = fixture();
    let case = f.case.clone().reparent(f.suite_id);
    assert_eq!(case, f.case);
  }

  #[test]
  fn to_steps_on_case_multi_keeps_ids_and_own_location() {
    let f = fixture();
    let steps = f.case.to_steps();

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].meta.id(), f.step_a_id);
    assert_eq!(steps[1].meta.id(), f.step_b_id);
    for step in &steps {
      assert!(matches!(step.meta.path(), TestPath::Step(suite, case, _) if suite == f.suite_id && case == f.case_id));
    }
    // The source case is untouched.
    assert_eq!(step_names(&f.case), vec!["step a", "step b"]);
  }

  #[test]
  fn to_steps_on_case_step_creates_a_single_step_with_a_fresh_id() {
    let f = case_step_fixture();
    let steps = f.case.to_steps();

    assert_eq!(steps.len(), 1);
    assert_ne!(steps[0].meta.id(), f.case_id);
    assert_eq!(steps[0].meta.name, SharedString::new("case step"));
    assert_eq!(steps[0].data, "payload");
    assert!(matches!(steps[0].meta.path(), TestPath::Step(suite, case, _) if suite == f.suite_id && case == f.case_id));
  }
}
