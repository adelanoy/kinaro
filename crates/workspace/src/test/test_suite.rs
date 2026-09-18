use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestMetadata, TestPath, test_case::TestCase};
use gpui_kit::SharedString;
use ki_project::FileTestSuite;
use log::{error, warn};
use std::fmt::Debug;
use uuid::Uuid;

/// A test suite: the root of the test tree.
///
/// A suite owns its metadata ([`TestMetadata`]) and a flat list of [`TestCase`]s.
/// Each case is either a *multi-step case* (a case that holds its own steps) or
/// a *case step* (a single step promoted directly to the suite level), so the
/// tree is addressed through [`TestPath`] paths rather than by index.
///
/// Suites are created with [`TestSuite::new`], or loaded from their on-disk
/// representation with [`TestSuite::from_file`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSuite {
  /// Metadata for the suite itself (id, name, description, disabled flag).
  ///
  /// `info.path` is always a [`TestPath::Suite`].
  pub meta: TestMetadata,
  /// The suite's direct children, in display order.
  pub cases: Vec<TestCase>,
}

impl TestSuite {
  /// Adds a new case relative to `path`: right after the case whose id
  /// matches (or that is its parent), or appended at the end when nothing
  /// matches.
  pub fn add_test_case(&mut self, path: &TestPath) {
    let position = self.cases.iter().position(|case| case.meta.path.is_parent(path));
    let name = ki_utils::next_available_name("new Case", self.cases.iter().map(|case| case.meta.name.clone()));
    match position {
      Some(ix) => self.cases.insert(ix + 1, TestCase::new_multi(self.meta.id(), name)),
      None => self.cases.push(TestCase::new_multi(self.meta.id(), name)),
    }
  }

  /// Adds a new step relative to `path`. When `path` addresses the
  /// suite itself, or a case whose `case_type` is a `CaseStep`, a new case
  /// step is appended to the suite; otherwise the step is delegated to the
  /// owning `CaseMulti`.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if `path` addresses a
  /// case or step whose owning case cannot be found by id in this suite.
  pub fn add_test_step(&mut self, path: &TestPath) -> ProjectResult<()> {
    if path.is_suite() {
      let name = ki_utils::next_available_name("New Step", self.cases.iter().map(|case| case.meta.name.clone()));
      self.cases.push(TestCase::new_step(self.meta.id(), name));
      return Ok(());
    }

    let case_id = path.case_id().expect("a non-Suite TestInfoId always has a case_id");
    let Some(ix) = self.cases.iter().position(|case| case.meta.id() == case_id) else {
      error!("TestSuite:add_test_step: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(*path));
    };

    if path.is_case() && self.cases[ix].is_case_step() {
      let name = ki_utils::next_available_name("New Step", self.cases.iter().map(|case| case.meta.name.clone()));
      self.cases.push(TestCase::new_step(self.meta.id(), name));
      Ok(())
    } else {
      self.cases[ix].add_test_step(path)
    }
  }

  /// Deletes the node addressed by `path` from this suite: the whole case
  /// when `path` addresses a case, or a single step delegated to its
  /// owning case otherwise.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no case in this suite is
  /// (or owns) the node addressed by `path`.
  pub(crate) fn delete_test(&mut self, path: &TestPath) -> ProjectResult<()> {
    let Some(ix) = self.cases.iter().position(|case| case.meta.path.is_parent(path)) else {
      warn!("TestSuite:delete_test: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(*path));
    };

    if path.is_case() {
      self.cases.remove(ix);
      Ok(())
    } else {
      self.cases[ix].delete_test(path)
    }
  }

  /// Duplicates this suite with a new random id and the given name. All
  /// cases (and their children) are also duplicated with new ids, but keep
  /// their original names.
  pub(crate) fn duplicate(&self, name: SharedString) -> Self {
    Self {
      meta: self.meta.duplicate(name),
      cases: self.cases.iter().map(|tc| tc.duplicate(tc.meta.name.clone())).collect(),
    }
  }

  /// Duplicates the case (or, for a step within a `CaseMulti`, delegates to
  /// that case) addressed by `path`, inserting the copy right after it
  /// with a name made unique against its siblings.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no case in this suite is
  /// (or owns) the node addressed by `path`.
  pub(crate) fn duplicate_child(&mut self, path: &TestPath) -> ProjectResult<()> {
    let Some(ix) = self.cases.iter().position(|case| case.meta.path.is_parent(path)) else {
      error!("TestSuite:duplicate_child: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(*path));
    };

    if matches!(path, TestPath::Case(_, _)) {
      let name = ki_utils::next_available_name(
        &self.cases[ix].meta.name,
        self.cases.iter().map(|case| case.meta.name.clone()),
      );
      let duplicate = self.cases[ix].duplicate(name);
      if ix == self.cases.len() - 1 {
        self.cases.push(duplicate);
      } else {
        self.cases.insert(ix + 1, duplicate);
      }
      Ok(())
    } else {
      self.cases[ix].duplicate_child(path)
    }
  }

  /// Builds a suite from its on-disk representation, reading ids from the
  /// file rather than generating new ones.
  pub(crate) fn from_file(file_test_suite: FileTestSuite) -> TestSuite {
    let id = file_test_suite.info.id;
    Self {
      meta: TestMetadata::new_suite(file_test_suite.info),
      cases: file_test_suite
        .cases
        .into_iter()
        .map(|case| TestCase::from_file(case, id))
        .collect(),
    }
  }

  /// Produces the serializable, on-disk representation of this suite.
  pub(crate) fn get_file(&self) -> FileTestSuite {
    FileTestSuite {
      info: self.meta.to_file(),
      cases: self.cases.iter().map(|case| case.get_file()).collect(),
    }
  }

  /// Looks up the [`TestMetadata`] of the case or step addressed by `path`.
  /// Always `None` when `path` addresses the suite itself, since the
  /// suite's own info isn't reachable through this lookup.
  #[allow(unused)]
  pub fn info_from_path(&self, path: &TestPath) -> Option<&TestMetadata> {
    let case = self.cases.iter().find(|case| case.meta.path.is_parent(path))?;
    match path {
      TestPath::Suite(_) => None,
      TestPath::Case(_, _) => Some(&case.meta),
      TestPath::Step(_, _, _) => case.info_from_path(path),
    }
  }

  /// Mutable counterpart to [`TestSuite::info_from_path`].
  pub fn info_mut_from_path(&mut self, path: &TestPath) -> Option<&mut TestMetadata> {
    let case = self.cases.iter_mut().find(|case| case.meta.path.is_parent(path))?;
    match path {
      TestPath::Suite(_) => None,
      TestPath::Case(_, _) => Some(&mut case.meta),
      TestPath::Step(_, _, _) => case.info_mut_from_path(path),
    }
  }

  /// Creates a new, empty suite with a freshly generated id.
  pub fn new(name: SharedString) -> Self {
    Self {
      meta: TestMetadata {
        path: TestPath::Suite(Uuid::new_v4()),
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
  use crate::error::ProjectError;
  use crate::test::TestPath;
  use crate::test::test_suite::TestSuite;
  use gpui_kit::SharedString;
  use ki_project::{FileTestCase, FileTestCaseType, FileTestMetadata, FileTestStep, FileTestSuite};
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
    assert!(f.suite.info_from_path(&TestPath::Suite(f.suite_id)).is_none());
    assert!(f.suite.info_from_path(&TestPath::Suite(Uuid::new_v4())).is_none());
  }

  #[test]
  fn info_from_path_case_multi() {
    let f = fixture();
    let info = f
      .suite
      .info_from_path(&TestPath::Case(f.suite_id, f.multi_id))
      .expect("multi case should be reachable");
    assert_eq!(info.id(), f.multi_id);
    assert_eq!(info.name, SharedString::new("multi case"));
    assert!(matches!(info.path(), TestPath::Case(suite, case) if suite == f.suite_id && case == f.multi_id));
  }

  #[test]
  fn info_from_path_case_multi_unknown() {
    let f = fixture();
    // Unknown case id, right suite.
    assert!(
      f.suite
        .info_from_path(&TestPath::Case(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    // Known case id, wrong suite.
    assert!(
      f.suite
        .info_from_path(&TestPath::Case(Uuid::new_v4(), f.multi_id))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_case_step() {
    let f = fixture();
    let info = f
      .suite
      .info_from_path(&TestPath::Case(f.suite_id, f.case_step_id))
      .expect("case step should be reachable");
    assert_eq!(info.id(), f.case_step_id);
    assert_eq!(info.name, SharedString::new("case step"));
    assert!(matches!(info.path(), TestPath::Case(suite, case) if suite == f.suite_id && case == f.case_step_id));
  }

  #[test]
  fn info_from_path_case_step_unknown() {
    let f = fixture();
    // Unknown case id, right suite.
    assert!(
      f.suite
        .info_from_path(&TestPath::Case(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    // Known case id, wrong suite.
    assert!(
      f.suite
        .info_from_path(&TestPath::Case(Uuid::new_v4(), f.case_step_id))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_step() {
    let f = fixture();
    let info = f
      .suite
      .info_from_path(&TestPath::Step(f.suite_id, f.multi_id, f.step_a_id))
      .expect("first step should be reachable");
    assert_eq!(info.id(), f.step_a_id);
    assert_eq!(info.name, SharedString::new("step a"));

    let info = f
      .suite
      .info_from_path(&TestPath::Step(f.suite_id, f.multi_id, f.step_b_id))
      .expect("second step should be reachable");
    assert_eq!(info.id(), f.step_b_id);
    assert_eq!(info.name, SharedString::new("step b"));
    assert!(
      matches!(info.path(), TestPath::Step(suite, case, step) if suite == f.suite_id && case == f.multi_id && step == f.step_b_id)
    );
  }

  #[test]
  fn info_from_path_step_unknown() {
    let f = fixture();
    // Owning case found, unknown step id.
    assert!(
      f.suite
        .info_from_path(&TestPath::Step(f.suite_id, f.multi_id, Uuid::new_v4()))
        .is_none()
    );
    // Known step id, unknown case id.
    assert!(
      f.suite
        .info_from_path(&TestPath::Step(f.suite_id, Uuid::new_v4(), f.step_a_id))
        .is_none()
    );
    // Known step id, wrong suite.
    assert!(
      f.suite
        .info_from_path(&TestPath::Step(Uuid::new_v4(), f.multi_id, f.step_a_id))
        .is_none()
    );
    // A case step has no children.
    assert!(
      f.suite
        .info_from_path(&TestPath::Step(f.suite_id, f.case_step_id, Uuid::new_v4()))
        .is_none()
    );
  }

  #[test]
  fn info_from_path_empty_suite() {
    let suite = TestSuite::new(SharedString::new("suite"));
    let suite_id = suite.meta.id();
    assert!(suite.info_from_path(&TestPath::Suite(suite_id)).is_none());
    assert!(suite.info_from_path(&TestPath::Case(suite_id, Uuid::new_v4())).is_none());
    assert!(
      suite
        .info_from_path(&TestPath::Step(suite_id, Uuid::new_v4(), Uuid::new_v4()))
        .is_none()
    );
  }

  #[test]
  fn info_mut_from_path_suite() {
    let mut f = fixture();
    assert!(f.suite.info_mut_from_path(&TestPath::Suite(f.suite_id)).is_none());
    assert!(f.suite.info_mut_from_path(&TestPath::Suite(Uuid::new_v4())).is_none());
  }

  #[test]
  fn info_mut_from_path_case_multi() {
    let mut f = fixture();
    let path = TestPath::Case(f.suite_id, f.multi_id);
    let info = f.suite.info_mut_from_path(&path).expect("multi case should be reachable");
    assert_eq!(info.id(), f.multi_id);
    info.name = SharedString::new("renamed case");
    info.disabled = true;

    let info = f.suite.info_from_path(&path).expect("multi case should be reachable");
    assert_eq!(info.name, SharedString::new("renamed case"));
    assert!(info.disabled);
  }

  #[test]
  fn info_mut_from_path_case_multi_unknown() {
    let mut f = fixture();
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Case(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Case(Uuid::new_v4(), f.multi_id))
        .is_none()
    );
  }

  #[test]
  fn info_mut_from_path_case_step() {
    let mut f = fixture();
    let path = TestPath::Case(f.suite_id, f.case_step_id);
    let info = f.suite.info_mut_from_path(&path).expect("case step should be reachable");
    assert_eq!(info.id(), f.case_step_id);
    info.name = SharedString::new("renamed case step");

    let info = f.suite.info_from_path(&path).expect("case step should be reachable");
    assert_eq!(info.name, SharedString::new("renamed case step"));
  }

  #[test]
  fn info_mut_from_path_case_step_unknown() {
    let mut f = fixture();
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Case(f.suite_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Case(Uuid::new_v4(), f.case_step_id))
        .is_none()
    );
  }

  #[test]
  fn info_mut_from_path_step() {
    let mut f = fixture();
    let path = TestPath::Step(f.suite_id, f.multi_id, f.step_b_id);
    let info = f.suite.info_mut_from_path(&path).expect("second step should be reachable");
    assert_eq!(info.id(), f.step_b_id);
    info.name = SharedString::new("renamed step");
    info.disabled = true;

    let info = f.suite.info_from_path(&path).expect("second step should be reachable");
    assert_eq!(info.name, SharedString::new("renamed step"));
    assert!(info.disabled);
    // The sibling step is untouched.
    let sibling = f
      .suite
      .info_from_path(&TestPath::Step(f.suite_id, f.multi_id, f.step_a_id))
      .expect("first step should be reachable");
    assert_eq!(sibling.name, SharedString::new("step a"));
    assert!(!sibling.disabled);
  }

  #[test]
  fn info_mut_from_path_step_unknown() {
    let mut f = fixture();
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Step(f.suite_id, f.multi_id, Uuid::new_v4()))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Step(f.suite_id, Uuid::new_v4(), f.step_a_id))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Step(Uuid::new_v4(), f.multi_id, f.step_a_id))
        .is_none()
    );
    assert!(
      f.suite
        .info_mut_from_path(&TestPath::Step(f.suite_id, f.case_step_id, Uuid::new_v4()))
        .is_none()
    );
  }

  fn case_names(suite: &TestSuite) -> Vec<String> {
    suite.cases.iter().map(|case| case.meta.name.to_string()).collect()
  }

  #[test]
  fn add_test_case_suite_selected_appends_at_end() {
    let mut f = fixture();
    f.suite.add_test_case(&TestPath::Suite(f.suite_id));
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step", "new Case"]);
  }

  #[test]
  fn add_test_case_after_case_multi() {
    let mut f = fixture();
    f.suite.add_test_case(&TestPath::Step(f.suite_id, f.multi_id, f.step_a_id));
    assert_eq!(case_names(&f.suite), vec!["multi case", "new Case", "case step"]);
  }

  #[test]
  fn add_test_case_after_case_step() {
    let mut f = fixture();
    f.suite.add_test_case(&TestPath::Case(f.suite_id, f.case_step_id));
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step", "new Case"]);
  }

  #[test]
  fn add_test_case_unknown_path_appends_at_end() {
    let mut f = fixture();
    // A path for a different suite entirely: no case in this suite is its parent.
    f.suite.add_test_case(&TestPath::Case(Uuid::new_v4(), Uuid::new_v4()));
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step", "new Case"]);
  }

  #[test]
  fn add_test_case_name_is_made_unique() {
    let mut f = fixture();
    f.suite.add_test_case(&TestPath::Suite(f.suite_id));
    f.suite.add_test_case(&TestPath::Suite(f.suite_id));
    assert_eq!(
      case_names(&f.suite),
      vec!["multi case", "case step", "new Case", "new Case_1"]
    );
  }

  #[test]
  fn delete_test_removes_case_multi() {
    let mut f = fixture();
    f.suite
      .delete_test(&TestPath::Case(f.suite_id, f.multi_id))
      .expect("delete_test should succeed");
    assert_eq!(case_names(&f.suite), vec!["case step"]);
  }

  #[test]
  fn delete_test_removes_case_step() {
    let mut f = fixture();
    f.suite
      .delete_test(&TestPath::Case(f.suite_id, f.case_step_id))
      .expect("delete_test should succeed");
    assert_eq!(case_names(&f.suite), vec!["multi case"]);
  }

  #[test]
  fn delete_test_removes_step_within_case_multi() {
    let mut f = fixture();
    f.suite
      .delete_test(&TestPath::Step(f.suite_id, f.multi_id, f.step_a_id))
      .expect("delete_test should succeed");
    // The case itself is untouched; only the step below it is removed.
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step"]);
    assert!(
      f.suite
        .info_from_path(&TestPath::Step(f.suite_id, f.multi_id, f.step_a_id))
        .is_none()
    );
    let sibling = f
      .suite
      .info_from_path(&TestPath::Step(f.suite_id, f.multi_id, f.step_b_id))
      .expect("remaining step should still be reachable");
    assert_eq!(sibling.name, SharedString::new("step b"));
  }

  #[test]
  fn delete_test_unknown_case_is_not_found() {
    let mut f = fixture();
    let err = f
      .suite
      .delete_test(&TestPath::Case(f.suite_id, Uuid::new_v4()))
      .expect_err("an unknown case id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
    assert_eq!(case_names(&f.suite), vec!["multi case", "case step"]);
  }

  #[test]
  fn delete_test_unknown_step_is_not_found() {
    let mut f = fixture();
    let err = f
      .suite
      .delete_test(&TestPath::Step(f.suite_id, f.multi_id, Uuid::new_v4()))
      .expect_err("an unknown step id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }
}
