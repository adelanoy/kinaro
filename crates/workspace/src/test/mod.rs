use crate::error::{ProjectError, ProjectResult};
use crate::test::test_case::TestCaseType;
use crate::{FileProjectMetadata, TestCase, TestStep, TestSuite};
use gpui_kit::{Context, EventEmitter, SharedString};
use ki_project::{FileTestMetadata, FileTestsContainer};
use ki_utils::Offset;
use log::warn;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

pub mod test_case;
pub mod test_step;
pub mod test_suite;

/// Enum wrapping the type of test Item (Suite, Case or Step), it's id and it's path.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TestPath {
  Suite(Uuid),
  Case(Uuid, Uuid),
  Step(Uuid, Uuid, Uuid),
}

impl TestPath {
  pub fn id(&self) -> Uuid {
    match *self {
      TestPath::Suite(id) => id,
      TestPath::Case(_, id) => id,
      TestPath::Step(_, _, id) => id,
    }
  }

  #[inline]
  pub fn is_case(&self) -> bool {
    matches!(self, TestPath::Case(_, _))
  }

  pub fn case_id(&self) -> Option<Uuid> {
    match *self {
      TestPath::Suite(_) => None,
      TestPath::Case(_, id) => Some(id),
      TestPath::Step(_, id, _) => Some(id),
    }
  }

  #[inline]
  pub fn is_step(&self) -> bool {
    matches!(self, TestPath::Step(_, _, _))
  }

  pub fn step_id(&self) -> Option<Uuid> {
    match *self {
      TestPath::Suite(_) | TestPath::Case(_, _) => None,
      TestPath::Step(_, _, id) => Some(id),
    }
  }

  #[inline]
  pub fn is_suite(&self) -> bool {
    matches!(self, TestPath::Suite(_))
  }

  pub fn suite_id(&self) -> Uuid {
    match *self {
      TestPath::Suite(id) => id,
      TestPath::Case(id, _) => id,
      TestPath::Step(id, _, _) => id,
    }
  }

  pub fn is_parent(&self, other: &TestPath) -> bool {
    match self {
      TestPath::Suite(_) => match other {
        TestPath::Suite(_) => self == other,
        TestPath::Case(suite_id, _) => self.suite_id() == *suite_id,
        TestPath::Step(suite_id, _, _) => self.suite_id() == *suite_id,
      },
      TestPath::Case(_, _) => match other {
        TestPath::Suite(_) => false,
        TestPath::Case(_, _) => self == other,
        TestPath::Step(suite_id, case_id, _) => self.suite_id() == *suite_id && self.case_id() == Some(*case_id),
      },
      TestPath::Step(_, _, _) => match other {
        TestPath::Suite(_) => false,
        TestPath::Case(_, _) => false,
        TestPath::Step(_, _, _) => self == other,
      },
    }
  }
}

impl Display for TestPath {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TestPath::Suite(suite_id) => f.write_fmt(format_args!("Suite:{}", suite_id)),
      TestPath::Case(suite_id, case_id) => f.write_fmt(format_args!("Suite:{}/Case:{}", suite_id, case_id)),
      TestPath::Step(suite_id, case_id, step_id) => {
        f.write_fmt(format_args!("Suite:{}/Case:{}/Step:{}", suite_id, case_id, step_id))
      }
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestMetadata {
  path: TestPath,
  pub name: SharedString,
  pub description: Option<SharedString>,
  pub disabled: bool,
}

impl TestMetadata {
  pub fn id(&self) -> Uuid {
    self.path.id()
  }

  pub fn path(&self) -> TestPath {
    self.path
  }

  pub(crate) fn new_suite(file_test_info: FileTestMetadata) -> Self {
    Self {
      path: TestPath::Suite(file_test_info.id),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub(crate) fn new_case(file_test_info: FileTestMetadata, suite_id: Uuid) -> Self {
    Self {
      path: TestPath::Case(suite_id, file_test_info.id),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub(crate) fn new_step(file_test_info: FileTestMetadata, suite_id: Uuid, case_id: Uuid) -> Self {
    Self {
      path: TestPath::Step(suite_id, case_id, file_test_info.id),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  pub(crate) fn duplicate(&self, name: SharedString) -> Self {
    let path = match self.path {
      TestPath::Suite(_) => TestPath::Suite(Uuid::new_v4()),
      TestPath::Case(suite_id, _) => TestPath::Case(suite_id, Uuid::new_v4()),
      TestPath::Step(suite_id, case_id, _) => TestPath::Step(suite_id, case_id, Uuid::new_v4()),
    };
    Self {
      path,
      name,
      description: self.description.clone(),
      disabled: self.disabled,
    }
  }

  pub(crate) fn to_file(&self) -> FileTestMetadata {
    FileTestMetadata {
      id: self.id(),
      name: self.name.to_string(),
      description: self
        .description
        .as_ref()
        .map(|s| ki_utils::to_multiline(s))
        .unwrap_or_default(),
      disabled: self.disabled,
    }
  }
}

/// Events raised bu the TestSContainer entity
pub enum TestsContainerEvent {
  /// The content of the container has changed
  TestsModified,
  /// The status of some node has changed (such as the node was collapsed/expanded in the tree)
  TreeNodesChanged,
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct TestsContainer {
  pub suites: Vec<TestSuite>,
  opened_tree_nodes: HashSet<Uuid>,
}

impl TestsContainer {
  pub fn add_test_case(&mut self, path: TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    let suite_id = path.suite_id();
    let Some(suite) = self.suites.iter_mut().find(|suite| suite.meta.id() == suite_id) else {
      warn!("TestsContainer:add_test_case: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(path));
    };
    let path = suite.add_test_case(path);
    // Update opened_tree_nodes
    self.opened_tree_nodes.insert(path.suite_id());

    cx.emit(TestsContainerEvent::TestsModified);
    Ok(path)
  }

  pub fn add_test_step(&mut self, path: TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    let suite_id = path.suite_id();
    let Some(suite) = self.suites.iter_mut().find(|suite| suite.meta.id() == suite_id) else {
      warn!("TestsContainer:add_test_step: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(path));
    };
    let path = suite.add_test_step(path)?;
    // Update opened_tree_nodes
    self.opened_tree_nodes.insert(path.suite_id());
    let (suite_ix, case_ix) = self.case_indexes(&path)?;
    if !self.suites[suite_ix].cases[case_ix].is_case_step() {
      self.opened_tree_nodes.insert(path.case_id().unwrap());
    }
    cx.emit(TestsContainerEvent::TestsModified);
    Ok(path)
  }

  pub fn add_test_suite(&mut self, path: Option<TestPath>, cx: &mut Context<Self>) -> TestPath {
    let position = match path {
      None => None,
      Some(path) => {
        let suite_id = path.suite_id();
        self.suites.iter().position(|suite| suite.meta.id() == suite_id)
      }
    };

    let name = ki_utils::next_available_name("New Suite", self.suites.iter().map(|suite| suite.meta.name.clone()));
    let new_suite = TestSuite::new(name);
    let path = new_suite.meta.path;
    match position {
      Some(ix) => self.suites.insert(ix + 1, new_suite),
      None => self.suites.push(new_suite),
    }
    cx.emit(TestsContainerEvent::TestsModified);
    path
  }

  /// Resolves the suite-list and case-list indices of the case at `path`,
  /// or, when `path` addresses a step, of the case that owns it.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no suite is a parent of `path`, or no
  /// case in that suite is (or owns) the node addressed by `path`.
  fn case_indexes(&self, path: &TestPath) -> ProjectResult<(usize, usize)> {
    let Some(suite_ix) = self.suites.iter().position(|suite| suite.meta.path.is_parent(path)) else {
      return Err(ProjectError::TestNotFound(*path));
    };
    let Some(case_ix) = self.suites[suite_ix]
      .cases
      .iter()
      .position(|case| case.meta.path.is_parent(path))
    else {
      return Err(ProjectError::TestNotFound(*path));
    };
    Ok((suite_ix, case_ix))
  }

  /// Resolves the suite-list, case-list and step-list indices of the step at
  /// `path`. Only valid when `path` addresses a step owned by a
  /// [`TestCaseType::CaseMulti`]; a [`TestCaseType::CaseStep`] has no steps
  /// to index into.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no suite is a parent of `path`, no
  /// case in that suite owns it, or the owning case is a
  /// [`TestCaseType::CaseStep`] (so it can't own a step by definition) or
  /// has no step matching `path`.
  fn step_indexes(&self, path: &TestPath) -> ProjectResult<(usize, usize, usize)> {
    let Some(suite_ix) = self.suites.iter().position(|suite| suite.meta.path.is_parent(path)) else {
      return Err(ProjectError::TestNotFound(*path));
    };
    let Some(case_ix) = self.suites[suite_ix]
      .cases
      .iter()
      .position(|case| case.meta.path.is_parent(path))
    else {
      return Err(ProjectError::TestNotFound(*path));
    };
    let Some(step_ix) = (match self.suites[suite_ix].cases[case_ix].case_type() {
      TestCaseType::CaseMulti { steps } => steps.iter().position(|step| step.meta.path == *path),
      TestCaseType::CaseStep { .. } => None,
    }) else {
      return Err(ProjectError::TestNotFound(*path));
    };
    Ok((suite_ix, case_ix, step_ix))
  }

  pub fn collapse_tree_node(&mut self, id: &Uuid, cx: &mut Context<Self>) {
    self.opened_tree_nodes.remove(id);
    cx.emit(TestsContainerEvent::TreeNodesChanged);
  }

  pub fn delete_test(&mut self, path: &TestPath, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(ix) = self.suites.iter().position(|suite| suite.meta.path.is_parent(path)) else {
      warn!("TestsContainer:delete_test: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(*path));
    };

    if path.is_suite() {
      self.suites.remove(ix);
    } else {
      self.suites[ix].delete_test(path)?;
    };
    cx.emit(TestsContainerEvent::TestsModified);
    Ok(())
  }

  pub fn duplicate_test(&mut self, path: &TestPath, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(ix) = self.suites.iter().position(|suite| suite.meta.path.is_parent(path)) else {
      warn!("TestsContainer:duplicate_test: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(*path));
    };

    if matches!(path, TestPath::Suite(_)) {
      let new_name = ki_utils::next_available_name(
        &self.suites[ix].meta.name,
        self.suites.iter().map(|suite| suite.meta.name.clone()),
      );
      let duplicate = self.suites[ix].duplicate(new_name);
      if ix == self.suites.len() - 1 {
        self.suites.push(duplicate);
      } else {
        self.suites.insert(ix + 1, duplicate);
      }
    } else {
      self.suites[ix].duplicate_child(path)?;
    }
    cx.emit(TestsContainerEvent::TestsModified);
    Ok(())
  }

  pub fn expand_tree_node(&mut self, id: Uuid, cx: &mut Context<Self>) {
    self.opened_tree_nodes.insert(id);
    cx.emit(TestsContainerEvent::TreeNodesChanged);
  }

  pub(super) fn from_file(file_container: FileTestsContainer, metadata: &FileProjectMetadata) -> Self {
    Self {
      suites: file_container.suites.into_iter().map(TestSuite::from_file).collect(),
      opened_tree_nodes: metadata.opened_tree_nodes.clone(),
    }
  }

  #[allow(unused)]
  pub fn info_from_path(&self, path: &TestPath) -> Option<&TestMetadata> {
    let suite = self.suites.iter().find(|suite| suite.meta.path.is_parent(path))?;
    match path {
      TestPath::Suite(_) => Some(&suite.meta),
      _ => suite.info_from_path(path),
    }
  }

  pub fn info_mut_from_path(&mut self, path: &TestPath) -> Option<&mut TestMetadata> {
    let suite = self.suites.iter_mut().find(|suite| suite.meta.path.is_parent(path))?;
    match path {
      TestPath::Suite(_) => Some(&mut suite.meta),
      _ => suite.info_mut_from_path(path),
    }
  }
  /// Moves the node at `path` one position toward `offset` among its
  /// siblings (case in its suite, step in its case, or suite in the
  /// container). A no-op when the node is already at that end of its
  /// sibling list.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if `path` doesn't address an existing
  /// node, or, for a step, if its owning case is a [`TestCaseType::CaseStep`]
  /// (so it has no steps to reorder).
  pub fn move_offset(&mut self, path: TestPath, offset: Offset, cx: &mut Context<Self>) -> ProjectResult<()> {
    fn offset_pos(ix: usize, max_ix: usize, offset: Offset) -> Option<usize> {
      match offset {
        Offset::Plus => {
          if ix < max_ix {
            Some(ix + 1)
          } else {
            None
          }
        }
        Offset::Minus => {
          if ix > 0 {
            Some(ix - 1)
          } else {
            None
          }
        }
      }
    }

    match path {
      TestPath::Suite(_) => {
        let Some(suite_ix) = self.suites.iter().position(|suite| suite.meta.path == path) else {
          return Err(ProjectError::TestNotFound(path));
        };
        if let Some(target_ix) = offset_pos(suite_ix, self.suites.len() - 1, offset) {
          self.suites.swap(suite_ix, target_ix);
          cx.emit(TestsContainerEvent::TestsModified);
        }
        Ok(())
      }
      TestPath::Case(_, _) => {
        let (suite_ix, case_ix) = self.case_indexes(&path)?;
        if let Some(target_ix) = offset_pos(case_ix, self.suites[suite_ix].cases.len() - 1, offset) {
          self.suites[suite_ix].cases.swap(case_ix, target_ix);
          cx.emit(TestsContainerEvent::TestsModified);
        }
        Ok(())
      }
      TestPath::Step(_, _, _) => {
        let (suite_ix, case_ix, step_ix) = self.step_indexes(&path)?;
        if let TestCaseType::CaseMulti { steps } = self.suites[suite_ix].cases[case_ix].case_type_mut()
          && let Some(target_ix) = offset_pos(step_ix, steps.len() - 1, offset)
        {
          steps.swap(step_ix, target_ix);
          cx.emit(TestsContainerEvent::TestsModified);
        }
        Ok(())
      }
    }
  }

  /// Moves the node at `from` to the node at `to`, e.g. for drag-and-drop
  /// reordering in the project tree. A no-op when `from == to`.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if `from` or `to` doesn't address an
  /// existing node.
  pub fn move_test(&mut self, from: TestPath, to: TestPath, cx: &mut Context<Self>) -> ProjectResult<()> {
    if from == to {
      return Ok(());
    }

    match to {
      TestPath::Suite(_) => self.move_to_suite(from, to),
      TestPath::Case(_, _) => self.move_to_case(from, to),
      TestPath::Step(_, _, _) => self.move_to_step(from, to),
    }?;

    cx.emit(TestsContainerEvent::TestsModified);
    Ok(())
  }

  /// Moves the node at `from` onto the suite at `to`: a suite is reordered
  /// in place, while a case or step is detached from its current parent and
  /// appended at the end of `to`'s case list (a step is first promoted to a
  /// [`TestCaseType::CaseStep`], keeping its own id as the new case's id).
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if `to` doesn't address an existing
  /// suite, or `from` doesn't address an existing node.
  fn move_to_suite(&mut self, from: TestPath, to: TestPath) -> ProjectResult<()> {
    let Some(to_suite_ix) = self.suites.iter().position(|suite| suite.meta.path == to) else {
      return Err(ProjectError::TestNotFound(to));
    };
    match from {
      TestPath::Suite(_) => {
        let Some(from_ix) = self.suites.iter().position(|suite| suite.meta.path == from) else {
          return Err(ProjectError::TestNotFound(from));
        };
        let suite = self.suites.remove(from_ix);
        self.suites.insert(to_suite_ix, suite);
        Ok(())
      }
      TestPath::Case(_, _) => {
        let (from_suite_ix, from_case_ix) = self.case_indexes(&from)?;
        let case = self.suites[from_suite_ix].cases.remove(from_case_ix).reparent(to.suite_id());
        self.suites[to_suite_ix].cases.push(case);
        Ok(())
      }
      TestPath::Step(_, _, id) => {
        let (from_suite_ix, from_case_ix) = self.case_indexes(&from)?;
        let Some(step) = self.suites[from_suite_ix].cases[from_case_ix].remove_step(id) else {
          return Err(ProjectError::TestNotFound(from));
        };
        let case_step = TestCase::from_step(step, to.suite_id());
        self.suites[to_suite_ix].cases.push(case_step);
        Ok(())
      }
    }
  }

  /// Moves the node at `from` onto the case at `to`. A suite source is a
  /// no-op (a suite can't become a case's child). A case source has its
  /// steps merged into `to`'s step list and is then removed — unless `to`
  /// is itself a [`TestCaseType::CaseStep`], which has no steps to merge
  /// into, in which case this is a no-op. A step source is detached from
  /// its case and inserted at `to`'s position as a new
  /// [`TestCaseType::CaseStep`], keeping its own id.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if `from` or `to` doesn't address an
  /// existing node.
  fn move_to_case(&mut self, from: TestPath, to: TestPath) -> ProjectResult<()> {
    match from {
      TestPath::Suite(_) => Ok(()),
      TestPath::Case(_, _) => {
        let (from_suite_ix, from_case_ix) = self.case_indexes(&from)?;
        let (to_suite_ix, to_case_ix) = self.case_indexes(&to)?;
        // Fighting the borrow checker: Cannot convert case and add it to its target in one go as it borrows twice self.suites
        // So, first clone/convert, add to target, and then remove original
        if !self.suites[to_suite_ix].cases[to_case_ix].is_case_step() {
          let new_steps: Vec<TestStep> = self.suites[from_suite_ix].cases[from_case_ix]
            .to_steps()
            .into_iter()
            .map(|step| step.reparent(to.suite_id(), to.case_id().expect("Has a case_id")))
            .collect();
          if let TestCaseType::CaseMulti { steps } = self.suites[to_suite_ix].cases[to_case_ix].case_type_mut() {
            steps.extend(new_steps);
          };
          self.suites[from_suite_ix].cases.remove(from_case_ix);
        }
        Ok(())
      }
      TestPath::Step(_, _, id) => {
        let (from_suite_ix, from_case_ix) = self.case_indexes(&from)?;
        let (to_suite_ix, to_case_ix) = self.case_indexes(&to)?;
        let Some(step) = self.suites[from_suite_ix].cases[from_case_ix].remove_step(id) else {
          return Err(ProjectError::TestNotFound(from));
        };
        let case_step = TestCase::from_step(step, to.suite_id());
        self.suites[to_suite_ix].cases.insert(to_case_ix, case_step);
        Ok(())
      }
    }
  }

  /// Moves the node at `from` onto the step at `to`. A suite source is a
  /// no-op. A case source is only valid when it's a
  /// [`TestCaseType::CaseStep`]: it's converted to a step and
  /// inserted at `to`'s position, then removed from its case. A step source
  /// is detached from its case and inserted at `to`'s position.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if `from` or `to` doesn't address an
  /// existing node.
  fn move_to_step(&mut self, from: TestPath, to: TestPath) -> ProjectResult<()> {
    match from {
      TestPath::Suite(_) => Ok(()),
      TestPath::Case(_, _) => {
        let (from_suite_ix, from_case_ix) = self.case_indexes(&from)?;
        if self.suites[from_suite_ix].cases[from_case_ix].is_case_step() {
          let (to_suite_ix, to_case_ix, to_step_ix) = self.step_indexes(&to)?;
          let step = self.suites[from_suite_ix].cases[from_case_ix]
            .to_steps()
            .remove(0)
            .reparent(to.suite_id(), to.case_id().expect("Has a case_id"));
          if let TestCaseType::CaseMulti { steps } = self.suites[to_suite_ix].cases[to_case_ix].case_type_mut() {
            steps.insert(to_step_ix, step);
          };
          self.suites[from_suite_ix].cases.remove(from_case_ix);
        }
        Ok(())
      }
      TestPath::Step(_, _, id) => {
        let (from_suite_ix, from_case_ix) = self.case_indexes(&from)?;
        let (to_suite_ix, to_case_ix, to_step_ix) = self.step_indexes(&to)?;
        let Some(mut step) = self.suites[from_suite_ix].cases[from_case_ix].remove_step(id) else {
          return Err(ProjectError::TestNotFound(from));
        };
        if let TestCaseType::CaseMulti { steps } = self.suites[to_suite_ix].cases[to_case_ix].case_type_mut() {
          step = step.reparent(to.suite_id(), to.case_id().expect("Has a case_id"));
          steps.insert(to_step_ix, step);
        };
        Ok(())
      }
    }
  }

  #[inline]
  pub fn opened_tree_nodes(&self) -> &HashSet<Uuid> {
    &self.opened_tree_nodes
  }

  pub fn rename_at(&mut self, path: &TestPath, name: SharedString, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(test_info) = self.info_mut_from_path(path) else {
      warn!("TestsContainer:rename_at: unknow path: {}", path);
      return Err(ProjectError::TestNotFound(*path));
    };

    if test_info.name != name {
      test_info.name = name;
      cx.emit(TestsContainerEvent::TestsModified);
    }

    Ok(())
  }

  pub fn switch_node_enable_status(&mut self, path: &TestPath, cx: &mut Context<Self>) {
    let Some(info) = self.info_mut_from_path(path) else {
      warn!("TestsContainer:switch_active_status: unknow path: {}", path);
      return;
    };
    info.disabled = !info.disabled;
    cx.emit(TestsContainerEvent::TestsModified);
  }

  pub fn to_file(&self) -> FileTestsContainer {
    FileTestsContainer {
      suites: self.suites.iter().map(|suite| suite.get_file()).collect(),
    }
  }
}

impl EventEmitter<TestsContainerEvent> for TestsContainer {}

#[cfg(test)]
mod tests {
  use crate::error::ProjectError;
  use crate::test::test_case::{TestCase, TestCaseType};
  use crate::test::test_suite::TestSuite;
  use crate::test::{TestMetadata, TestPath, TestsContainer};
  use gpui_kit::{AppContext, Entity, SharedString, TestAppContext};
  use ki_project::{FileTestCase, FileTestCaseType, FileTestMetadata, FileTestStep, FileTestSuite};
  use ki_utils::Offset;
  use uuid::Uuid;

  #[test]
  fn test_path_id() {
    let id = Uuid::new_v4();
    // id
    let path = TestPath::Suite(id);
    assert_eq!(path.id(), id);
    let path = TestPath::Case(Uuid::new_v4(), id);
    assert_eq!(path.id(), id);
    let path = TestPath::Step(Uuid::new_v4(), Uuid::new_v4(), id);
    assert_eq!(path.id(), id);
  }

  #[test]
  fn test_path_test_suite_id() {
    let id = Uuid::new_v4();
    // test_suite id
    let path = TestPath::Suite(id);
    assert_eq!(path.suite_id(), id);
    let path = TestPath::Case(id, Uuid::new_v4());
    assert_eq!(path.suite_id(), id);
    let path = TestPath::Step(id, Uuid::new_v4(), Uuid::new_v4());
    assert_eq!(path.suite_id(), id);
  }

  #[test]
  fn test_path_test_case_id() {
    let id = Uuid::new_v4();
    // test_suite id
    let path = TestPath::Suite(id);
    assert_eq!(path.case_id(), None);
    let path = TestPath::Case(Uuid::new_v4(), id);
    assert_eq!(path.case_id(), Some(id));
    let path = TestPath::Step(Uuid::new_v4(), id, Uuid::new_v4());
    assert_eq!(path.case_id(), Some(id));
  }

  #[test]
  fn test_path_test_step_id() {
    let id = Uuid::new_v4();
    // test_suite id
    let path = TestPath::Suite(id);
    assert_eq!(path.step_id(), None);
    let path = TestPath::Case(Uuid::new_v4(), id);
    assert_eq!(path.step_id(), None);
    let path = TestPath::Step(Uuid::new_v4(), Uuid::new_v4(), id);
    assert_eq!(path.step_id(), Some(id));
  }

  #[test]
  fn test_path_is_parent_suite() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let path = TestPath::Suite(suite_id);
    // Assert parent
    assert!(path.is_parent(&path));
    assert!(path.is_parent(&TestPath::Case(suite_id, case_id)));
    assert!(path.is_parent(&TestPath::Step(suite_id, case_id, step_id)));
    // Assert not parent
    assert!(!path.is_parent(&TestPath::Suite(Uuid::new_v4())));
    assert!(!path.is_parent(&TestPath::Case(Uuid::new_v4(), case_id)));
    assert!(!path.is_parent(&TestPath::Step(Uuid::new_v4(), case_id, step_id)));
  }

  #[test]
  fn test_path_is_parent_case() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let path = TestPath::Case(suite_id, case_id);
    // Assert parent
    assert!(path.is_parent(&TestPath::Case(suite_id, case_id)));
    assert!(path.is_parent(&TestPath::Step(suite_id, case_id, step_id)));
    // Assert not parent
    assert!(!path.is_parent(&TestPath::Suite(suite_id)));
    assert!(!path.is_parent(&TestPath::Case(suite_id, Uuid::new_v4())));
    assert!(!path.is_parent(&TestPath::Case(Uuid::new_v4(), case_id)));
    assert!(!path.is_parent(&TestPath::Step(Uuid::new_v4(), case_id, step_id)));
  }

  #[test]
  fn test_path_is_parent_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let path = TestPath::Step(suite_id, case_id, step_id);
    // Assert parent
    assert!(path.is_parent(&TestPath::Step(suite_id, case_id, step_id)));
    // Assert not parent
    assert!(!path.is_parent(&TestPath::Suite(suite_id)));
    assert!(!path.is_parent(&TestPath::Case(suite_id, Uuid::new_v4())));
    assert!(!path.is_parent(&TestPath::Step(suite_id, case_id, Uuid::new_v4())));
    assert!(!path.is_parent(&TestPath::Step(suite_id, Uuid::new_v4(), step_id)));
  }

  #[test]
  fn test_path() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();

    let mut info = TestMetadata {
      path: TestPath::Suite(suite_id),
      name: Default::default(),
      description: None,
      disabled: false,
    };
    assert_eq!(suite_id, info.id());
    info.path = TestPath::Case(suite_id, case_id);
    assert_eq!(case_id, info.id());
    info.path = TestPath::Step(suite_id, case_id, step_id);
    assert_eq!(step_id, info.id());
  }

  #[test]
  fn test_info_new_suite() {
    let suite_id = Uuid::new_v4();
    let file_info = FileTestMetadata {
      id: suite_id,
      name: suite_id.to_string(),
      description: vec![suite_id.to_string()],
      disabled: true,
    };

    let info = TestMetadata::new_suite(file_info);
    match info.path() {
      TestPath::Suite(suite) if suite == suite_id => {}
      _ => panic!("TestInfo::new_suite failed with non-matching ids"),
    }
    assert_eq!(suite_id, info.id());
    assert_eq!(format!("{}", suite_id), info.name.to_string());
    assert_eq!(format!("{}", suite_id), info.description.map(|s| s.to_string()).unwrap());
    assert!(info.disabled);
  }

  #[test]
  fn test_info_new_case() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let file_info = FileTestMetadata {
      id: case_id,
      name: case_id.to_string(),
      description: vec![case_id.to_string()],
      disabled: true,
    };

    let info = TestMetadata::new_case(file_info, suite_id);
    match info.path() {
      TestPath::Case(suite, case) if suite == suite_id && case == case_id => {}
      _ => panic!("TestInfo::new_suite failed with non-matching ids"),
    }
    assert_eq!(case_id, info.id());
    assert_eq!(format!("{}", case_id), info.name.to_string());
    assert_eq!(format!("{}", case_id), info.description.map(|s| s.to_string()).unwrap());
    assert!(info.disabled);
    assert!(matches!(info.path, TestPath::Case(_, _)));
  }

  #[test]
  fn test_info_new_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let file_info = FileTestMetadata {
      id: step_id,
      name: step_id.to_string(),
      description: vec![step_id.to_string()],
      disabled: true,
    };

    let info = TestMetadata::new_step(file_info, suite_id, case_id);
    match info.path() {
      TestPath::Step(suite, case, step) if suite == suite_id && case == case_id && step == step_id => {}
      _ => panic!("TestInfo::new_suite failed with non-matching ids"),
    }
    assert_eq!(step_id, info.id());
    assert_eq!(format!("{}", step_id), info.name.to_string());
    assert_eq!(format!("{}", step_id), info.description.map(|s| s.to_string()).unwrap());
    assert!(info.disabled);
    assert!(matches!(info.path, TestPath::Step(_, _, _)));
  }

  #[test]
  fn test_info_duplicate_suite() {
    let suite_id = Uuid::new_v4();
    let info = TestMetadata {
      path: TestPath::Suite(suite_id),
      name: SharedString::new("Suite"),
      description: Some(SharedString::new("description")),
      disabled: true,
    };
    let new_name = SharedString::new("Suite duplicate");
    let duplicate = info.duplicate(new_name.clone());
    assert!(matches!(duplicate.path, TestPath::Suite(suite) if suite != suite_id));
    assert_eq!(duplicate.name, new_name);
    assert_eq!(duplicate.description, info.description);
    assert_eq!(duplicate.disabled, info.disabled);
  }

  #[test]
  fn test_info_duplicate_case() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let info = TestMetadata {
      path: TestPath::Case(suite_id, case_id),
      name: SharedString::new("Case"),
      description: Some(SharedString::new("description")),
      disabled: true,
    };
    let new_name = SharedString::new("Case duplicate");
    let duplicate = info.duplicate(new_name.clone());
    assert!(matches!(duplicate.path, TestPath::Case(suite, case) if suite == suite_id && case != case_id));
    assert_eq!(duplicate.name, new_name);
    assert_eq!(duplicate.description, info.description);
    assert_eq!(duplicate.disabled, info.disabled);
  }

  #[test]
  fn test_info_duplicate_step() {
    let suite_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let info = TestMetadata {
      path: TestPath::Step(suite_id, case_id, step_id),
      name: SharedString::new("Step"),
      description: Some(SharedString::new("description")),
      disabled: true,
    };
    let new_name = SharedString::new("Step duplicate");
    let duplicate = info.duplicate(new_name.clone());
    assert!(
      matches!(duplicate.path, TestPath::Step(suite, case, step) if suite == suite_id && case == case_id && step != step_id)
    );
    assert_eq!(duplicate.name, new_name);
    assert_eq!(duplicate.description, info.description);
    assert_eq!(duplicate.disabled, info.disabled);
  }

  /// A container holding two suites, each with one multi-step case (two
  /// steps) and one case step, built through [`TestSuite::from_file`] so
  /// that every id is known upfront.
  struct ContainerFixture {
    container: TestsContainer,
    suite_a_id: Uuid,
    suite_b_id: Uuid,
    case_a_multi_id: Uuid,
    step_a1_id: Uuid,
    step_a2_id: Uuid,
    case_a_step_id: Uuid,
    case_b_multi_id: Uuid,
    step_b1_id: Uuid,
    step_b2_id: Uuid,
    case_b_step_id: Uuid,
  }

  fn file_info(id: Uuid, name: &str) -> FileTestMetadata {
    FileTestMetadata {
      id,
      name: name.to_string(),
      description: vec![],
      disabled: false,
    }
  }

  fn file_suite(id: Uuid, name: &str, multi_id: Uuid, step_ids: [Uuid; 2], case_step_id: Uuid) -> FileTestSuite {
    FileTestSuite {
      info: file_info(id, name),
      cases: vec![
        FileTestCase {
          info: file_info(multi_id, &format!("{name} multi case")),
          case_type: FileTestCaseType::CaseMulti {
            steps: vec![
              FileTestStep {
                info: file_info(step_ids[0], &format!("{name} step 1")),
                data: String::new(),
              },
              FileTestStep {
                info: file_info(step_ids[1], &format!("{name} step 2")),
                data: String::new(),
              },
            ],
          },
        },
        FileTestCase {
          info: file_info(case_step_id, &format!("{name} case step")),
          case_type: FileTestCaseType::CaseStep {
            data: format!("{name} case step data"),
          },
        },
      ],
    }
  }

  fn container_fixture() -> ContainerFixture {
    let suite_a_id = Uuid::new_v4();
    let suite_b_id = Uuid::new_v4();
    let case_a_multi_id = Uuid::new_v4();
    let step_a1_id = Uuid::new_v4();
    let step_a2_id = Uuid::new_v4();
    let case_a_step_id = Uuid::new_v4();
    let case_b_multi_id = Uuid::new_v4();
    let step_b1_id = Uuid::new_v4();
    let step_b2_id = Uuid::new_v4();
    let case_b_step_id = Uuid::new_v4();

    let suite_a = TestSuite::from_file(file_suite(
      suite_a_id,
      "suite a",
      case_a_multi_id,
      [step_a1_id, step_a2_id],
      case_a_step_id,
    ));
    let suite_b = TestSuite::from_file(file_suite(
      suite_b_id,
      "suite b",
      case_b_multi_id,
      [step_b1_id, step_b2_id],
      case_b_step_id,
    ));

    let container = TestsContainer {
      suites: vec![suite_a, suite_b],
      ..Default::default()
    };

    ContainerFixture {
      container,
      suite_a_id,
      suite_b_id,
      case_a_multi_id,
      step_a1_id,
      step_a2_id,
      case_a_step_id,
      case_b_multi_id,
      step_b1_id,
      step_b2_id,
      case_b_step_id,
    }
  }

  fn case_names(suite: &TestSuite) -> Vec<String> {
    suite.cases.iter().map(|case| case.meta.name.to_string()).collect()
  }

  fn step_ids(case: &TestCase) -> Vec<Uuid> {
    match case.case_type() {
      TestCaseType::CaseMulti { steps } => steps.iter().map(|step| step.meta.id()).collect(),
      TestCaseType::CaseStep { .. } => vec![],
    }
  }

  #[test]
  fn case_indexes_finds_case_by_exact_path() {
    let f = container_fixture();
    let (suite_ix, case_ix) = f
      .container
      .case_indexes(&TestPath::Case(f.suite_a_id, f.case_a_multi_id))
      .expect("case a multi should be found");
    assert_eq!(f.container.suites[suite_ix].meta.id(), f.suite_a_id);
    assert_eq!(f.container.suites[suite_ix].cases[case_ix].meta.id(), f.case_a_multi_id);
  }

  #[test]
  fn case_indexes_finds_the_case_owning_a_step() {
    let f = container_fixture();
    let (suite_ix, case_ix) = f
      .container
      .case_indexes(&TestPath::Step(f.suite_a_id, f.case_a_multi_id, f.step_a1_id))
      .expect("the step's parent case should be found");
    assert_eq!(f.container.suites[suite_ix].cases[case_ix].meta.id(), f.case_a_multi_id);
  }

  #[test]
  fn case_indexes_unknown_suite_is_not_found() {
    let f = container_fixture();
    let err = f
      .container
      .case_indexes(&TestPath::Case(Uuid::new_v4(), f.case_a_multi_id))
      .expect_err("an unknown suite id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn case_indexes_unknown_case_is_not_found() {
    let f = container_fixture();
    let err = f
      .container
      .case_indexes(&TestPath::Case(f.suite_a_id, Uuid::new_v4()))
      .expect_err("an unknown case id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn step_indexes_finds_step() {
    let f = container_fixture();
    let (suite_ix, case_ix, step_ix) = f
      .container
      .step_indexes(&TestPath::Step(f.suite_a_id, f.case_a_multi_id, f.step_a2_id))
      .expect("step a2 should be found");
    assert_eq!(step_ids(&f.container.suites[suite_ix].cases[case_ix])[step_ix], f.step_a2_id);
  }

  #[test]
  fn step_indexes_unknown_step_is_not_found() {
    let f = container_fixture();
    let err = f
      .container
      .step_indexes(&TestPath::Step(f.suite_a_id, f.case_a_multi_id, Uuid::new_v4()))
      .expect_err("an unknown step id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn step_indexes_on_case_step_is_not_found() {
    let f = container_fixture();
    // `case_a_step_id` is a CaseStep: it has no steps to index into.
    let err = f
      .container
      .step_indexes(&TestPath::Step(f.suite_a_id, f.case_a_step_id, Uuid::new_v4()))
      .expect_err("a case step has no children");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_suite_unknown_target_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_suite(TestPath::Suite(f.suite_a_id), TestPath::Suite(Uuid::new_v4()))
      .expect_err("an unknown target suite should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_suite_unknown_source_suite_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_suite(TestPath::Suite(Uuid::new_v4()), TestPath::Suite(f.suite_b_id))
      .expect_err("an unknown source suite should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_suite_reorders_suites_backward() {
    // A backward move (removing a later suite doesn't shift the target's
    // index): the known forward-move off-by-one is deliberately not
    // exercised here.
    let mut container = TestsContainer {
      suites: vec![
        TestSuite::new(SharedString::new("A")),
        TestSuite::new(SharedString::new("B")),
        TestSuite::new(SharedString::new("C")),
      ],
      ..Default::default()
    };
    let a_id = container.suites[0].meta.id();
    let c_id = container.suites[2].meta.id();

    container
      .move_to_suite(TestPath::Suite(c_id), TestPath::Suite(a_id))
      .expect("move should succeed");

    let names: Vec<String> = container.suites.iter().map(|s| s.meta.name.to_string()).collect();
    assert_eq!(names, vec!["C", "A", "B"]);
  }

  #[test]
  fn move_to_suite_moves_a_case_and_reparents_it() {
    let mut f = container_fixture();
    f.container
      .move_to_suite(TestPath::Case(f.suite_a_id, f.case_a_multi_id), TestPath::Suite(f.suite_b_id))
      .expect("move should succeed");

    // Removed from its original suite.
    assert_eq!(case_names(&f.container.suites[0]), vec!["suite a case step"]);
    // Appended at the end of the target suite, keeping its own id.
    let moved = f.container.suites[1].cases.last().expect("case should have moved");
    assert_eq!(moved.meta.id(), f.case_a_multi_id);
    assert!(matches!(moved.meta.path(), TestPath::Case(suite, case) if suite == f.suite_b_id && case == f.case_a_multi_id));
    // Its steps are reparented under the new suite too, keeping their own ids.
    let TestCaseType::CaseMulti { steps } = moved.case_type() else {
      panic!("expected a CaseMulti");
    };
    assert!(
      matches!(steps[0].meta.path(), TestPath::Step(suite, case, id) if suite == f.suite_b_id && case == f.case_a_multi_id && id == f.step_a1_id)
    );
    assert!(
      matches!(steps[1].meta.path(), TestPath::Step(suite, case, id) if suite == f.suite_b_id && case == f.case_a_multi_id && id == f.step_a2_id)
    );
  }

  #[test]
  fn move_to_suite_unknown_source_case_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_suite(TestPath::Case(f.suite_a_id, Uuid::new_v4()), TestPath::Suite(f.suite_b_id))
      .expect_err("an unknown case id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_suite_promotes_a_step_to_a_case_step() {
    let mut f = container_fixture();
    f.container
      .move_to_suite(
        TestPath::Step(f.suite_a_id, f.case_a_multi_id, f.step_a1_id),
        TestPath::Suite(f.suite_b_id),
      )
      .expect("move should succeed");

    // Removed from its original case.
    assert_eq!(step_ids(&f.container.suites[0].cases[0]), vec![f.step_a2_id]);

    // Appended to the target suite as a new case step, keeping the step's own id.
    let moved = f.container.suites[1].cases.last().expect("step should have been promoted");
    assert!(moved.is_case_step());
    assert_eq!(moved.meta.id(), f.step_a1_id);
    assert!(matches!(moved.meta.path(), TestPath::Case(suite, case) if suite == f.suite_b_id && case == f.step_a1_id));
  }

  #[test]
  fn move_to_suite_unknown_source_step_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_suite(
        TestPath::Step(f.suite_a_id, f.case_a_multi_id, Uuid::new_v4()),
        TestPath::Suite(f.suite_b_id),
      )
      .expect_err("an unknown step id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_case_suite_source_is_a_no_op() {
    let mut f = container_fixture();
    f.container
      .move_to_case(TestPath::Suite(f.suite_a_id), TestPath::Case(f.suite_b_id, f.case_b_multi_id))
      .expect("a suite source should be a no-op, not an error");
    assert_eq!(
      case_names(&f.container.suites[0]),
      vec!["suite a multi case", "suite a case step"]
    );
  }

  #[test]
  fn move_to_case_merges_case_multi_steps_into_target() {
    let mut f = container_fixture();
    f.container
      .move_to_case(
        TestPath::Case(f.suite_a_id, f.case_a_multi_id),
        TestPath::Case(f.suite_b_id, f.case_b_multi_id),
      )
      .expect("move should succeed");

    // The source case is gone.
    assert_eq!(case_names(&f.container.suites[0]), vec!["suite a case step"]);
    // Its steps are appended to the target case, reparented under it.
    let target = &f.container.suites[1].cases[0];
    assert_eq!(step_ids(target), vec![f.step_b1_id, f.step_b2_id, f.step_a1_id, f.step_a2_id]);
    let TestCaseType::CaseMulti { steps } = target.case_type() else {
      panic!("expected a CaseMulti");
    };
    for step in &steps[2..] {
      assert!(matches!(step.meta.path(), TestPath::Step(suite, case, _) if suite == f.suite_b_id && case == f.case_b_multi_id));
    }
  }

  #[test]
  fn move_to_case_converts_case_step_into_a_step_and_appends() {
    let mut f = container_fixture();
    f.container
      .move_to_case(
        TestPath::Case(f.suite_a_id, f.case_a_step_id),
        TestPath::Case(f.suite_b_id, f.case_b_multi_id),
      )
      .expect("move should succeed");

    assert_eq!(case_names(&f.container.suites[0]), vec!["suite a multi case"]);
    let target = &f.container.suites[1].cases[0];
    let TestCaseType::CaseMulti { steps } = target.case_type() else {
      panic!("expected a CaseMulti");
    };
    assert_eq!(steps.len(), 3);
    let appended = steps.last().expect("the case step should have been appended");
    assert_ne!(appended.meta.id(), f.case_a_step_id);
    assert_eq!(appended.meta.name, SharedString::new("suite a case step"));
    assert_eq!(appended.data, "suite a case step data");
  }

  #[test]
  fn move_to_case_case_to_case_step_target_is_a_no_op() {
    let mut f = container_fixture();
    f.container
      .move_to_case(
        TestPath::Case(f.suite_a_id, f.case_a_multi_id),
        TestPath::Case(f.suite_b_id, f.case_b_step_id),
      )
      .expect("a case-step target should be a no-op, not an error");
    // Nothing moved: the source case is still there, the target still a case step.
    assert_eq!(
      case_names(&f.container.suites[0]),
      vec!["suite a multi case", "suite a case step"]
    );
    assert!(f.container.suites[1].cases[1].is_case_step());
  }

  #[test]
  fn move_to_case_unknown_source_case_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_case(
        TestPath::Case(f.suite_a_id, Uuid::new_v4()),
        TestPath::Case(f.suite_b_id, f.case_b_multi_id),
      )
      .expect_err("an unknown source case should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_case_unknown_target_case_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_case(
        TestPath::Case(f.suite_a_id, f.case_a_multi_id),
        TestPath::Case(f.suite_b_id, Uuid::new_v4()),
      )
      .expect_err("an unknown target case should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_case_moves_a_step_into_a_case_inserted_at_position() {
    let mut f = container_fixture();
    f.container
      .move_to_case(
        TestPath::Step(f.suite_a_id, f.case_a_multi_id, f.step_a1_id),
        TestPath::Case(f.suite_b_id, f.case_b_multi_id),
      )
      .expect("move should succeed");

    assert_eq!(step_ids(&f.container.suites[0].cases[0]), vec![f.step_a2_id]);
    // Inserted right before the target case (case_b_multi_id is suite_b.cases[0]).
    assert_eq!(
      case_names(&f.container.suites[1]),
      vec!["suite a step 1", "suite b multi case", "suite b case step"]
    );
    let inserted = &f.container.suites[1].cases[0];
    assert!(inserted.is_case_step());
    assert_eq!(inserted.meta.id(), f.step_a1_id);
  }

  #[test]
  fn move_to_case_unknown_source_step_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_case(
        TestPath::Step(f.suite_a_id, f.case_a_multi_id, Uuid::new_v4()),
        TestPath::Case(f.suite_b_id, f.case_b_multi_id),
      )
      .expect_err("an unknown source step should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_step_suite_source_is_a_no_op() {
    let mut f = container_fixture();
    f.container
      .move_to_step(
        TestPath::Suite(f.suite_a_id),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, f.step_b1_id),
      )
      .expect("a suite source should be a no-op, not an error");
    assert_eq!(step_ids(&f.container.suites[1].cases[0]), vec![f.step_b1_id, f.step_b2_id]);
  }

  #[test]
  fn move_to_step_case_multi_source_is_a_no_op() {
    let mut f = container_fixture();
    f.container
      .move_to_step(
        TestPath::Case(f.suite_a_id, f.case_a_multi_id),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, f.step_b1_id),
      )
      .expect("a CaseMulti source should be a no-op, not an error");
    assert_eq!(
      case_names(&f.container.suites[0]),
      vec!["suite a multi case", "suite a case step"]
    );
    assert_eq!(step_ids(&f.container.suites[1].cases[0]), vec![f.step_b1_id, f.step_b2_id]);
  }

  #[test]
  fn move_to_step_converts_a_case_step_into_a_step_at_position() {
    let mut f = container_fixture();
    f.container
      .move_to_step(
        TestPath::Case(f.suite_a_id, f.case_a_step_id),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, f.step_b1_id),
      )
      .expect("move should succeed");

    assert_eq!(case_names(&f.container.suites[0]), vec!["suite a multi case"]);
    let TestCaseType::CaseMulti { steps } = f.container.suites[1].cases[0].case_type() else {
      panic!("expected a CaseMulti");
    };
    assert_eq!(steps.len(), 3);
    // Inserted right before step_b1 (its position in the target's step list).
    assert_ne!(steps[0].meta.id(), f.case_a_step_id);
    assert_eq!(steps[0].meta.name, SharedString::new("suite a case step"));
    assert_eq!(steps[0].data, "suite a case step data");
    // Reparented under the target suite/case.
    assert!(matches!(steps[0].meta.path(), TestPath::Step(suite, case, _) if suite == f.suite_b_id && case == f.case_b_multi_id));
    assert_eq!(steps[1].meta.id(), f.step_b1_id);
    assert_eq!(steps[2].meta.id(), f.step_b2_id);
  }

  #[test]
  fn move_to_step_unknown_target_step_is_not_found_for_case_source() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_step(
        TestPath::Case(f.suite_a_id, f.case_a_step_id),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, Uuid::new_v4()),
      )
      .expect_err("an unknown target step should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_step_unknown_source_case_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_step(
        TestPath::Case(f.suite_a_id, Uuid::new_v4()),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, f.step_b1_id),
      )
      .expect_err("an unknown source case should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_step_moves_a_step_across_cases_inserted_at_position() {
    let mut f = container_fixture();
    f.container
      .move_to_step(
        TestPath::Step(f.suite_a_id, f.case_a_multi_id, f.step_a1_id),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, f.step_b2_id),
      )
      .expect("move should succeed");

    assert_eq!(step_ids(&f.container.suites[0].cases[0]), vec![f.step_a2_id]);
    // Inserted right before step_b2 (its position in the target's step list).
    assert_eq!(
      step_ids(&f.container.suites[1].cases[0]),
      vec![f.step_b1_id, f.step_a1_id, f.step_b2_id]
    );
    // Reparented under the target suite/case.
    let TestCaseType::CaseMulti { steps } = f.container.suites[1].cases[0].case_type() else {
      panic!("expected a CaseMulti");
    };
    assert!(
      matches!(steps[1].meta.path(), TestPath::Step(suite, case, id) if suite == f.suite_b_id && case == f.case_b_multi_id && id == f.step_a1_id)
    );
  }

  #[test]
  fn move_to_step_unknown_source_step_is_not_found() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_step(
        TestPath::Step(f.suite_a_id, f.case_a_multi_id, Uuid::new_v4()),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, f.step_b1_id),
      )
      .expect_err("an unknown source step should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[test]
  fn move_to_step_unknown_target_step_is_not_found_for_step_source() {
    let mut f = container_fixture();
    let err = f
      .container
      .move_to_step(
        TestPath::Step(f.suite_a_id, f.case_a_multi_id, f.step_a1_id),
        TestPath::Step(f.suite_b_id, f.case_b_multi_id, Uuid::new_v4()),
      )
      .expect_err("an unknown target step should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  /// `move_offset` requires a live `Context<TestsContainer>` (it calls
  /// `cx.emit`), unlike the cx-free helpers exercised above, so these tests
  /// run under `#[gpui_kit::test]` and drive the container through a real
  /// entity.
  fn container_entity(container: TestsContainer, cx: &mut TestAppContext) -> Entity<TestsContainer> {
    cx.new(|_| container)
  }

  fn three_suites_container() -> TestsContainer {
    TestsContainer {
      suites: vec![
        TestSuite::new(SharedString::new("A")),
        TestSuite::new(SharedString::new("B")),
        TestSuite::new(SharedString::new("C")),
      ],
      ..Default::default()
    }
  }

  fn suite_names(container: &TestsContainer) -> Vec<String> {
    container.suites.iter().map(|suite| suite.meta.name.to_string()).collect()
  }

  #[gpui_kit::test]
  fn move_offset_plus_reorders_suites(cx: &mut TestAppContext) {
    let container = three_suites_container();
    let a_id = container.suites[0].meta.id();
    let entity = container_entity(container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Suite(a_id), Offset::Plus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| assert_eq!(suite_names(container), vec!["B", "A", "C"]));
  }

  #[gpui_kit::test]
  fn move_offset_plus_suite_already_last_is_a_no_op(cx: &mut TestAppContext) {
    let container = three_suites_container();
    let c_id = container.suites[2].meta.id();
    let entity = container_entity(container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Suite(c_id), Offset::Plus, cx)
      })
      .expect("a no-op move should still succeed");

    entity.read_with(cx, |container, _| assert_eq!(suite_names(container), vec!["A", "B", "C"]));
  }

  #[gpui_kit::test]
  fn move_offset_plus_unknown_suite_is_not_found(cx: &mut TestAppContext) {
    let entity = container_entity(container_fixture().container, cx);

    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Suite(Uuid::new_v4()), Offset::Plus, cx)
      })
      .expect_err("an unknown suite id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[gpui_kit::test]
  fn move_offset_minus_reorders_suites(cx: &mut TestAppContext) {
    let container = three_suites_container();
    let c_id = container.suites[2].meta.id();
    let entity = container_entity(container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Suite(c_id), Offset::Minus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| assert_eq!(suite_names(container), vec!["A", "C", "B"]));
  }

  #[gpui_kit::test]
  fn move_offset_minus_suite_already_first_is_a_no_op(cx: &mut TestAppContext) {
    let container = three_suites_container();
    let a_id = container.suites[0].meta.id();
    let entity = container_entity(container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Suite(a_id), Offset::Minus, cx)
      })
      .expect("a no-op move should still succeed");

    entity.read_with(cx, |container, _| assert_eq!(suite_names(container), vec!["A", "B", "C"]));
  }

  #[gpui_kit::test]
  fn move_offset_minus_unknown_suite_is_not_found(cx: &mut TestAppContext) {
    let entity = container_entity(container_fixture().container, cx);

    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Suite(Uuid::new_v4()), Offset::Minus, cx)
      })
      .expect_err("an unknown suite id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[gpui_kit::test]
  fn move_offset_plus_reorders_cases(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Case(suite_a_id, case_a_multi_id), Offset::Plus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(
        case_names(&container.suites[0]),
        vec!["suite a case step", "suite a multi case"]
      );
    });
  }

  /// Regression test: `move_offset` must resolve the case's own position
  /// among its siblings, not the position of its parent suite in the suite
  /// list. `suite_b` is `suites[1]`, so a bug conflating the two indices
  /// would misjudge boundaries or swap with the wrong sibling for any case
  /// here, even though `case_b_multi_id` is itself `cases[0]`.
  #[gpui_kit::test]
  fn move_offset_plus_reorders_cases_in_a_later_suite(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_b_id = f.suite_b_id;
    let case_b_multi_id = f.case_b_multi_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Case(suite_b_id, case_b_multi_id), Offset::Plus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(
        case_names(&container.suites[1]),
        vec!["suite b case step", "suite b multi case"]
      );
    });
  }

  #[gpui_kit::test]
  fn move_offset_plus_case_already_last_is_a_no_op(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_step_id = f.case_a_step_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Case(suite_a_id, case_a_step_id), Offset::Plus, cx)
      })
      .expect("a no-op move should still succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(
        case_names(&container.suites[0]),
        vec!["suite a multi case", "suite a case step"]
      );
    });
  }

  #[gpui_kit::test]
  fn move_offset_plus_unknown_case_is_not_found(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let entity = container_entity(f.container, cx);

    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Case(suite_a_id, Uuid::new_v4()), Offset::Plus, cx)
      })
      .expect_err("an unknown case id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[gpui_kit::test]
  fn move_offset_minus_reorders_cases(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_step_id = f.case_a_step_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Case(suite_a_id, case_a_step_id), Offset::Minus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(
        case_names(&container.suites[0]),
        vec!["suite a case step", "suite a multi case"]
      );
    });
  }

  #[gpui_kit::test]
  fn move_offset_minus_case_already_first_is_a_no_op(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Case(suite_a_id, case_a_multi_id), Offset::Minus, cx)
      })
      .expect("a no-op move should still succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(
        case_names(&container.suites[0]),
        vec!["suite a multi case", "suite a case step"]
      );
    });
  }

  #[gpui_kit::test]
  fn move_offset_minus_unknown_case_is_not_found(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let entity = container_entity(f.container, cx);

    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Case(suite_a_id, Uuid::new_v4()), Offset::Minus, cx)
      })
      .expect_err("an unknown case id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[gpui_kit::test]
  fn move_offset_plus_reorders_steps(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let step_a1_id = f.step_a1_id;
    let step_a2_id = f.step_a2_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_multi_id, step_a1_id), Offset::Plus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(step_ids(&container.suites[0].cases[0]), vec![step_a2_id, step_a1_id]);
    });
  }

  /// Regression test: same as `move_offset_plus_reorders_cases_in_a_later_suite`,
  /// but for the step branch, which must key off the step's own position,
  /// not its parent suite's position in the suite list.
  #[gpui_kit::test]
  fn move_offset_plus_reorders_steps_in_a_later_suite(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_b_id = f.suite_b_id;
    let case_b_multi_id = f.case_b_multi_id;
    let step_b1_id = f.step_b1_id;
    let step_b2_id = f.step_b2_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_b_id, case_b_multi_id, step_b1_id), Offset::Plus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(step_ids(&container.suites[1].cases[0]), vec![step_b2_id, step_b1_id]);
    });
  }

  #[gpui_kit::test]
  fn move_offset_plus_step_already_last_is_a_no_op(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let step_a1_id = f.step_a1_id;
    let step_a2_id = f.step_a2_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_multi_id, step_a2_id), Offset::Plus, cx)
      })
      .expect("a no-op move should still succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(step_ids(&container.suites[0].cases[0]), vec![step_a1_id, step_a2_id]);
    });
  }

  #[gpui_kit::test]
  fn move_offset_plus_unknown_step_is_not_found(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let entity = container_entity(f.container, cx);

    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_multi_id, Uuid::new_v4()), Offset::Plus, cx)
      })
      .expect_err("an unknown step id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[gpui_kit::test]
  fn move_offset_plus_case_step_has_no_steps_to_reorder(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_step_id = f.case_a_step_id;
    let entity = container_entity(f.container, cx);

    // `case_a_step_id` is a CaseStep: it has no steps to index into.
    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_step_id, Uuid::new_v4()), Offset::Plus, cx)
      })
      .expect_err("a case step has no children");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[gpui_kit::test]
  fn move_offset_minus_reorders_steps(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let step_a1_id = f.step_a1_id;
    let step_a2_id = f.step_a2_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_multi_id, step_a2_id), Offset::Minus, cx)
      })
      .expect("move should succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(step_ids(&container.suites[0].cases[0]), vec![step_a2_id, step_a1_id]);
    });
  }

  #[gpui_kit::test]
  fn move_offset_minus_step_already_first_is_a_no_op(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let step_a1_id = f.step_a1_id;
    let step_a2_id = f.step_a2_id;
    let entity = container_entity(f.container, cx);

    entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_multi_id, step_a1_id), Offset::Minus, cx)
      })
      .expect("a no-op move should still succeed");

    entity.read_with(cx, |container, _| {
      assert_eq!(step_ids(&container.suites[0].cases[0]), vec![step_a1_id, step_a2_id]);
    });
  }

  #[gpui_kit::test]
  fn move_offset_minus_unknown_step_is_not_found(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_multi_id = f.case_a_multi_id;
    let entity = container_entity(f.container, cx);

    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_multi_id, Uuid::new_v4()), Offset::Minus, cx)
      })
      .expect_err("an unknown step id should fail");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }

  #[gpui_kit::test]
  fn move_offset_minus_case_step_has_no_steps_to_reorder(cx: &mut TestAppContext) {
    let f = container_fixture();
    let suite_a_id = f.suite_a_id;
    let case_a_step_id = f.case_a_step_id;
    let entity = container_entity(f.container, cx);

    // `case_a_step_id` is a CaseStep: it has no steps to index into.
    let err = entity
      .update(cx, |container, cx| {
        container.move_offset(TestPath::Step(suite_a_id, case_a_step_id, Uuid::new_v4()), Offset::Minus, cx)
      })
      .expect_err("a case step has no children");
    assert!(matches!(err, ProjectError::TestNotFound(_)));
  }
}
