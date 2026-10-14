use crate::error::{ProjectError, ProjectResult};
use crate::test::{
  TestMetadata, TestMetadataOwner, TestPath, TestsContainer, TestsContainerEvent, test_case::TestCase, test_step::TestStep,
};
use gpui_kit::component::Icon;
use gpui_kit::{App, AppContext, ClipboardString, Context, Entity, SharedString, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileSuiteChild, FileTestCase, FileTestStep, FileTestSuite};
use ki_utils::{CASE, Offset, STEP};
use log::{debug, warn};
use std::cmp::min;
use std::collections::HashMap;
use std::fmt::Debug;
use uuid::Uuid;

/// A direct child of a [`TestSuite`]: a test case, or a test step located directly in the suite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestSuiteChild {
  /// A test case, holding its own steps (addressed by a [`TestPath::Case`])
  Case(Entity<TestCase>),
  /// A test step located directly in the suite (addressed by a [`TestPath::SuiteStep`])
  Step(Entity<TestStep>),
}

impl TestSuiteChild {
  /// Returns the path of this child followed by the paths of its descendants, in display order.
  pub(crate) fn all_paths(&self, cx: &App) -> Vec<TestPath> {
    match self {
      TestSuiteChild::Case(case) => case.read(cx).all_paths(cx),
      TestSuiteChild::Step(step) => vec![step.read(cx).path()],
    }
  }

  /// Returns a deep copy of this child named `name`, placed in the suite `suite_path` goes through.
  /// The copy and its descendants get fresh ids, while the descendants keep their names.
  fn duplicate(&self, name: SharedString, suite_path: &TestPath, cx: &mut App) -> Self {
    match self {
      TestSuiteChild::Case(case) => {
        let copy = case.update(cx, |case, cx| case.duplicate(name, suite_path.suite_id(), cx));
        TestSuiteChild::Case(cx.new(|_| copy))
      }
      TestSuiteChild::Step(step) => {
        let copy = step.read(cx).duplicate(name, &TestPath::Suite(suite_path.suite_id()));
        TestSuiteChild::Step(cx.new(|_| copy))
      }
    }
  }

  /// Builds a child of the suite `suite_id` from its on-disk representation. With `new_id`, the
  /// child and its descendants get freshly generated ids; otherwise ids are read from the file.
  fn from_file(
    file_child: FileSuiteChild,
    suite_id: Uuid,
    new_id: bool,
    container: WeakEntity<TestsContainer>,
    cx: &mut App,
  ) -> Self {
    match file_child {
      FileSuiteChild::Case(case) => TestSuiteChild::Case(cx.new(|cx| TestCase::from_file(case, suite_id, new_id, container, cx))),
      FileSuiteChild::Step(step) => {
        TestSuiteChild::Step(cx.new(|_| TestStep::from_file(step, &TestPath::Suite(suite_id), container, new_id)))
      }
    }
  }

  /// Produces the serializable, on-disk representation of this child.
  fn get_file(&self, cx: &App) -> FileSuiteChild {
    match self {
      TestSuiteChild::Case(case) => FileSuiteChild::Case(case.read(cx).get_file(cx)),
      TestSuiteChild::Step(step) => FileSuiteChild::Step(step.read(cx).get_file()),
    }
  }

  /// Returns the display name of this child.
  pub(crate) fn name(&self, cx: &App) -> SharedString {
    match self {
      TestSuiteChild::Case(case) => case.read(cx).name(),
      TestSuiteChild::Step(step) => step.read(cx).name(),
    }
  }

  /// Returns the path addressing this child in the test tree.
  fn path(&self, cx: &App) -> TestPath {
    match self {
      TestSuiteChild::Case(case) => case.read(cx).path(),
      TestSuiteChild::Step(step) => step.read(cx).path(),
    }
  }

  /// Moves this child, and its descendants, under the suite `parent` goes through, keeping their
  /// ids. Returns the paths that changed, as `old path -> new path` (including unchanged paths,
  /// mapped to themselves).
  fn reparent(&self, parent: &TestPath, cx: &mut App) -> HashMap<TestPath, TestPath> {
    match self {
      TestSuiteChild::Case(case) => case.update(cx, |case, cx| case.reparent(parent, true, cx)),
      TestSuiteChild::Step(step) => {
        let suite_path = TestPath::Suite(parent.suite_id());
        let (old, new) = step.update(cx, |step, _| step.reparent(&suite_path, true));
        HashMap::from([(old, new)])
      }
    }
  }
}

/// A test suite: the root of the test tree.
///
/// A suite owns its metadata ([`TestMetadata`]) and a flat list of children ([`TestSuiteChild`]):
/// test cases, holding their own steps, and test steps located directly in the suite. The tree is
/// addressed through [`TestPath`] paths rather than by index.
///
/// Suites are created with [`TestSuite::new`], or loaded from their on-disk
/// representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSuite {
  /// The container holding this suite, notified of the changes made to the suite
  container: WeakEntity<TestsContainer>,
  /// Metadata for the suite itself (id, name, description, disabled flag).
  ///
  /// `meta.path` is always a [`TestPath::Suite`].
  pub meta: TestMetadata,
  /// The suite's direct children, in display order.
  pub children: Vec<TestSuiteChild>,
}

impl TestSuite {
  /// Adds a new suite step (a delay of 1000 ms) relative to `path`: right after the child `path`
  /// points to or goes through, or appended at the end when `path` is the suite itself or no child
  /// matches. The new step gets a fresh id and a unique name. Returns its path.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the new step.
  pub fn add_new_delay_step(&mut self, path: TestPath, cx: &mut Context<Self>) -> TestPath {
    let position = self.position_after(&path, cx).unwrap_or(self.children.len());
    let name = ki_utils::next_available_name("new Delay", self.children.iter().map(|child| child.name(cx)));
    let new_step = TestStep::new_delay(&self.meta.path, name, self.container.clone());
    let path = new_step.meta.path;
    self.children.insert(position, TestSuiteChild::Step(cx.new(|_| new_step)));
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(path)));
    path
  }

  /// Adds a new, empty case relative to `path`: right after the child `path` points to or goes
  /// through, or appended at the end when `path` is the suite itself or no child matches. The new
  /// case gets a fresh id and a unique name. Returns its path.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the new case.
  pub fn add_new_test_case(&mut self, path: TestPath, cx: &mut Context<Self>) -> TestPath {
    let position = self.position_after(&path, cx).unwrap_or(self.children.len());
    let name = ki_utils::next_available_name("new Case", self.children.iter().map(|child| child.name(cx)));
    let new_case = TestCase::new(self.meta.id(), name, self.container.clone());
    let path = new_case.meta.path;
    self.children.insert(position, TestSuiteChild::Case(cx.new(|_| new_case)));
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(path)));
    path
  }

  /// Returns the path of this suite followed by the paths of all of its descendants, in display order.
  pub fn all_paths(&self, cx: &App) -> Vec<TestPath> {
    let mut paths = Vec::new();
    paths.push(self.meta.path);
    paths.extend(self.children.iter().flat_map(|child| child.all_paths(cx)));
    paths
  }

  /// Returns the case of this suite that `path` points to or goes through
  /// (the owning case of a step), or `None` when no case matches.
  pub(crate) fn case_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestCase>> {
    self.children.iter().find_map(|child| match child {
      TestSuiteChild::Case(case) if case.read(cx).meta.path.is_parent(path) => Some(case.clone()),
      _ => None,
    })
  }

  /// Returns a deep copy of this suite named `name`. The copy and all of its descendants get fresh
  /// ids, while the descendants keep their names.
  pub(crate) fn duplicate(&self, name: SharedString, cx: &mut Context<Self>) -> Self {
    let path = TestPath::Suite(Uuid::new_v4());
    let children = self
      .children
      .iter()
      .map(|child| {
        let name = child.name(cx);
        child.duplicate(name, &path, cx)
      })
      .collect();
    Self {
      container: self.container.clone(),
      meta: TestMetadata {
        path,
        name,
        description: self.meta.description.clone(),
        disabled: self.meta.disabled,
      },
      children,
    }
  }

  /// Duplicates the child (case or suite step) at `path`, along with its descendants, and inserts
  /// the copy right after it. The copy gets fresh ids and a unique name. Returns the path of the copy.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no child of this suite is at `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the copy.
  pub fn duplicate_child(&mut self, path: &TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    let Some(ix) = self.children.iter().position(|child| child.path(cx) == *path) else {
      return Err(ProjectError::TestNotFound(*path));
    };
    let child = &self.children[ix];
    let new_name = ki_utils::next_available_name(&child.name(cx), self.children.iter().map(|child| child.name(cx)));
    let copy = child.duplicate(new_name, &self.meta.path, cx);
    let new_path = copy.path(cx);
    self.children.insert(ix + 1, copy);
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
    Ok(new_path)
  }

  /// Builds a suite from its on-disk representation. With `new_id`, the suite and all of its
  /// descendants get freshly generated ids (e.g. when pasted); otherwise ids are read from the file.
  pub(crate) fn from_file(
    file_test_suite: FileTestSuite,
    container: WeakEntity<TestsContainer>,
    new_id: bool,
    cx: &mut Context<Self>,
  ) -> Self {
    let meta = if new_id {
      TestMetadata::new_suite_from_file(file_test_suite.info)
    } else {
      TestMetadata::suite_from_file(file_test_suite.info)
    };
    let id = meta.id();
    Self {
      container: container.clone(),
      meta,
      children: file_test_suite
        .children
        .into_iter()
        .map(|child| TestSuiteChild::from_file(child, id, new_id, container.clone(), cx))
        .collect(),
    }
  }

  /// Produces the serializable, on-disk representation of this suite.
  pub fn get_file(&self, cx: &App) -> FileTestSuite {
    FileTestSuite {
      info: self.meta.to_file(),
      children: self.children.iter().map(|child| child.get_file(cx)).collect(),
    }
  }

  /// Inserts `child` relative to `path`: appended at the end when `path` is the suite itself,
  /// otherwise right after the child `path` points to or goes through (or at the end when none
  /// matches). The child, and its descendants, are reparented under this suite, keeping their ids.
  /// Returns the paths that changed, as `old path -> new path`.
  ///
  /// # Errors
  /// [`ProjectError::NoOp`] if `path` isn't in this suite.
  pub(crate) fn insert_child(
    &mut self,
    child: TestSuiteChild,
    path: &TestPath,
    cx: &mut Context<Self>,
  ) -> ProjectResult<HashMap<TestPath, TestPath>> {
    if self.meta.id() != path.suite_id() {
      return Err(ProjectError::NoOp);
    }
    let updated_paths = child.reparent(&self.meta.path, cx);
    let ix = if path.is_suite() {
      self.children.len()
    } else {
      self.position_after(path, cx).unwrap_or(self.children.len())
    };
    self.children.insert(ix, child);

    Ok(updated_paths)
  }

  /// Creates a new, empty suite with a freshly generated id.
  pub fn new(name: SharedString, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      meta: TestMetadata {
        path: TestPath::Suite(Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      children: vec![],
    }
  }

  /// Moves the child (case or suite step) at `path` one position down ([`Offset::Plus`]) or up
  /// ([`Offset::Minus`]). A no-op when it is already the last, or the first, child.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no child of this suite is at `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the child moved.
  pub fn offset_child(&mut self, path: TestPath, offset: Offset, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(pos) = self.children.iter().position(|child| child.path(cx) == path) else {
      return Err(ProjectError::TestNotFound(path));
    };
    let new_pos = match offset {
      Offset::Plus => min(pos + 1, self.children.len() - 1),
      Offset::Minus => pos.saturating_sub(1),
    };
    if pos != new_pos {
      self.children.swap(pos, new_pos);
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    }
    Ok(())
  }

  /// Pastes the case or step held in `cb_str`, as told by its metadata ([`CASE`] or [`STEP`]),
  /// relative to `target`: appended at the end when `target` is the suite itself, otherwise right
  /// after the child `target` points to or goes through. A step is pasted as a suite step. The
  /// pasted node and its descendants get fresh ids. Nothing is pasted, and a warning is logged, if
  /// `target` isn't in this suite or the clipboard doesn't hold a valid case or step.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the pasted node.
  pub fn paste_from_clipboard(&mut self, cb_str: &ClipboardString, target: TestPath, cx: &mut Context<Self>) {
    // Pasted on the suite itself: appended at the end, otherwise right after the targeted child
    let ix = if target == self.meta.path {
      Some(self.children.len())
    } else {
      self.position_after(&target, cx)
    };
    let Some(ix) = ix else {
      warn!("TestSuite::paste_from_clipboard: Attempted to paste on a unknown path: {target}");
      return;
    };
    let file_child = match cb_str.metadata.as_deref() {
      Some(CASE) => serde_json::from_str::<FileTestCase>(&cb_str.text).map(FileSuiteChild::Case),
      Some(STEP) => serde_json::from_str::<FileTestStep>(&cb_str.text).map(FileSuiteChild::Step),
      _ => return,
    };
    let Ok(file_child) = file_child else {
      warn!("Could not paste in Suite: Clipboard contains invalid Json");
      debug!("Invalid Json: {}", cb_str.text);
      return;
    };
    let child = TestSuiteChild::from_file(file_child, self.meta.id(), true, self.container.clone(), cx);
    let new_path = child.path(cx);
    self.children.insert(ix, child);
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
  }

  /// Returns the index right after the child `path` points to or goes through, or `None` when no
  /// child matches.
  fn position_after(&self, path: &TestPath, cx: &App) -> Option<usize> {
    self
      .children
      .iter()
      .position(|child| child.path(cx).is_parent(path))
      .map(|ix| ix + 1)
  }

  /// Removes the child (case or suite step) at `path` and returns it, or `None` when no child of
  /// this suite is at `path`. No event is emitted: the caller is responsible for notifying the
  /// removal.
  pub fn remove_child(&mut self, path: &TestPath, cx: &mut Context<Self>) -> Option<TestSuiteChild> {
    let ix = self.children.iter().position(|child| child.path(cx) == *path)?;
    Some(self.children.remove(ix))
  }

  /// Returns the step `path` points to, located directly in this suite or in one of its cases, or
  /// `None` when no step matches.
  pub(crate) fn step_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestStep>> {
    match path {
      TestPath::SuiteStep(_, _) => self.children.iter().find_map(|child| match child {
        TestSuiteChild::Step(step) if step.read(cx).meta.path == *path => Some(step.clone()),
        _ => None,
      }),
      TestPath::Step(_, _, _) => self.case_from_path(path, cx)?.read(cx).step_from_path(path, cx),
      TestPath::Suite(_) | TestPath::Case(_, _) => None,
    }
  }
}

impl TestMetadataOwner for TestSuite {
  #[inline]
  fn id(&self) -> Uuid {
    self.meta.id()
  }

  #[inline]
  fn path(&self) -> TestPath {
    self.meta.path
  }

  fn icon(&self) -> impl Into<Icon> {
    IconAsset::TestSuite
  }

  #[inline]
  fn name(&self) -> SharedString {
    self.meta.name.clone()
  }

  fn set_name(&mut self, new_name: SharedString, cx: &mut Context<Self>) {
    if new_name == self.meta.name {
      return;
    }
    self.meta.name = new_name.clone();
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    cx.notify();
  }

  #[inline]
  fn description(&self) -> Option<SharedString> {
    self.meta.description.clone()
  }

  fn set_description(&mut self, new_description: Option<SharedString>, cx: &mut Context<Self>) {
    if new_description == self.meta.description {
      return;
    }
    self.meta.description = new_description.clone();
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    cx.notify();
  }

  #[inline]
  fn disabled(&self) -> bool {
    self.meta.disabled
  }

  fn switch_disabled(&mut self, cx: &mut Context<Self>) {
    self.meta.disabled = !self.meta.disabled;
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    cx.notify();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::step_delay::DelayStep;
  use crate::test::test_fixtures::*;
  use crate::test::test_step::StepData;
  use gpui_kit::TestAppContext;
  use ki_utils::SUITE;

  #[gpui_kit::test]
  fn new_suite_is_empty_and_enabled(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let suite = TestSuite::new("Suite".into(), tests.downgrade());
    assert!(suite.path().is_suite());
    assert_eq!(suite.name(), "Suite");
    assert_eq!(suite.description(), None);
    assert!(!suite.disabled());
    assert!(suite.children.is_empty());
    assert_ne!(TestSuite::new("Suite".into(), tests.downgrade()).id(), suite.id());
  }

  #[gpui_kit::test]
  fn all_paths_lists_the_suite_then_its_descendants(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    assert_eq!(
      subtree(&tests, p.suite_a, cx),
      vec![p.suite_a, p.case_a, p.step_a1, p.step_a2, p.suite_step_a]
    );
  }

  #[gpui_kit::test]
  fn get_file_round_trips(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite_a = suite(&tests, p.suite_a, cx);
    let saved = cx.read(|cx| suite_a.read(cx).get_file(cx));
    assert_eq!(
      serde_json::to_value(saved).unwrap(),
      serde_json::to_value(&p.file().suites[0]).unwrap()
    );
  }

  // Adding children

  #[gpui_kit::test]
  fn add_new_test_case_on_the_suite_appends(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let new_path = suite(&tests, p.suite_a, cx).update(cx, |suite, cx| suite.add_new_test_case(p.suite_a, cx));
    assert_eq!(child_names(&tests, p.suite_a, cx), vec!["Case A", "Suite step A", "new Case"]);
    assert!(new_path.is_case() && p.suite_a.is_parent(&new_path));
    assert!(case(&tests, new_path, cx).read_with(cx, |case, _| case.steps().is_empty()));
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(new_path)]);
  }

  #[gpui_kit::test]
  fn add_new_test_case_inserts_after_the_child_of_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite = suite(&tests, p.suite_a, cx);
    suite.update(cx, |suite, cx| suite.add_new_test_case(p.step_a2, cx));
    assert_eq!(child_names(&tests, p.suite_a, cx), vec!["Case A", "new Case", "Suite step A"]);
    suite.update(cx, |suite, cx| suite.add_new_test_case(p.suite_step_a, cx));
    assert_eq!(
      child_names(&tests, p.suite_a, cx),
      vec!["Case A", "new Case", "Suite step A", "new Case_1"]
    );
  }

  #[gpui_kit::test]
  fn add_new_delay_step_adds_a_suite_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    let first = suite.update(cx, |suite, cx| suite.add_new_delay_step(p.case_a, cx));
    let second = suite.update(cx, |suite, cx| suite.add_new_delay_step(p.suite_a, cx));
    assert_eq!(
      child_names(&tests, p.suite_a, cx),
      vec!["Case A", "new Delay", "Suite step A", "new Delay_1"]
    );
    assert!(matches!(first, TestPath::SuiteStep(suite, _) if suite == p.suite_a.id()));
    assert!(step(&tests, first, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(1000))));
    assert_eq!(
      *events.borrow(),
      vec![TestsContainerEvent::TestAdded(first), TestsContainerEvent::TestAdded(second)]
    );
  }

  // Duplicating children

  #[gpui_kit::test]
  fn duplicate_case_inserts_a_copy_after_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let copy = suite(&tests, p.suite_a, cx)
      .update(cx, |suite, cx| suite.duplicate_child(&p.case_a, cx))
      .unwrap();
    assert_eq!(child_names(&tests, p.suite_a, cx), vec!["Case A", "Case A_1", "Suite step A"]);
    assert_eq!(step_names(&tests, copy, cx), vec!["Step A1", "Step A2"]);
    let copy_paths = subtree(&tests, copy, cx);
    let original_paths = subtree(&tests, p.case_a, cx);
    for path in &copy_paths {
      assert!(copy.is_parent(path));
      assert!(original_paths.iter().all(|original| original.id() != path.id()));
    }
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(copy)]);
  }

  #[gpui_kit::test]
  fn duplicate_suite_step_keeps_its_data(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let copy = suite(&tests, p.suite_b, cx)
      .update(cx, |suite, cx| suite.duplicate_child(&p.suite_step_b, cx))
      .unwrap();
    assert_eq!(
      child_names(&tests, p.suite_b, cx),
      vec!["Case B", "Suite step B", "Suite step B_1"]
    );
    assert!(matches!(copy, TestPath::SuiteStep(suite, step) if suite == p.suite_b.id() && step != p.suite_step_b.id()));
    assert!(step(&tests, copy, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(500))));
  }

  #[gpui_kit::test]
  fn duplicate_child_requires_a_child_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite = suite(&tests, p.suite_a, cx);
    for path in [p.step_a1, p.suite_a, TestPath::Case(p.suite_a.id(), Uuid::new_v4())] {
      let result = suite.update(cx, |suite, cx| suite.duplicate_child(&path, cx));
      assert!(matches!(result, Err(ProjectError::TestNotFound(not_found)) if not_found == path));
    }
  }

  // Offsetting children

  #[gpui_kit::test]
  fn offset_child_swaps_with_its_neighbour(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    suite
      .update(cx, |suite, cx| suite.offset_child(p.case_a, Offset::Plus, cx))
      .unwrap();
    assert_eq!(child_names(&tests, p.suite_a, cx), vec!["Suite step A", "Case A"]);
    suite
      .update(cx, |suite, cx| suite.offset_child(p.suite_step_a, Offset::Plus, cx))
      .unwrap();
    assert_eq!(child_names(&tests, p.suite_a, cx), vec!["Case A", "Suite step A"]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 2]);
  }

  #[gpui_kit::test]
  fn offset_child_at_the_bounds_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    suite
      .update(cx, |suite, cx| suite.offset_child(p.case_a, Offset::Minus, cx))
      .unwrap();
    suite
      .update(cx, |suite, cx| suite.offset_child(p.suite_step_a, Offset::Plus, cx))
      .unwrap();
    assert_eq!(child_names(&tests, p.suite_a, cx), vec!["Case A", "Suite step A"]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn offset_child_unknown_is_not_found(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let result = suite(&tests, p.suite_a, cx).update(cx, |suite, cx| suite.offset_child(p.case_b, Offset::Plus, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == p.case_b));
  }

  // Removing children

  #[gpui_kit::test]
  fn remove_child_returns_it_without_notifying(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    let case_a = case(&tests, p.case_a, cx);
    let suite_step_a = step(&tests, p.suite_step_a, cx);
    assert_eq!(
      suite.update(cx, |suite, cx| suite.remove_child(&p.case_a, cx)),
      Some(TestSuiteChild::Case(case_a))
    );
    assert_eq!(
      suite.update(cx, |suite, cx| suite.remove_child(&p.suite_step_a, cx)),
      Some(TestSuiteChild::Step(suite_step_a))
    );
    assert!(child_names(&tests, p.suite_a, cx).is_empty());
    assert_eq!(suite.update(cx, |suite, cx| suite.remove_child(&p.case_a, cx)), None);
    assert!(events.borrow().is_empty());
  }

  // Inserting children

  #[gpui_kit::test]
  fn insert_case_reparents_it_and_its_steps(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = suite(&tests, p.suite_a, cx).update(cx, |suite, cx| suite.remove_child(&p.case_a, cx).unwrap());
    let suite_b = suite(&tests, p.suite_b, cx);
    let updated = suite_b
      .update(cx, |suite, cx| suite.insert_child(case_a, &p.case_b, cx))
      .unwrap();
    assert_eq!(child_names(&tests, p.suite_b, cx), vec!["Case B", "Case A", "Suite step B"]);
    let sb = p.suite_b.id();
    assert_eq!(
      updated,
      [
        (p.case_a, TestPath::Case(sb, p.case_a.id())),
        (p.step_a1, TestPath::Step(sb, p.case_a.id(), p.step_a1.id())),
        (p.step_a2, TestPath::Step(sb, p.case_a.id(), p.step_a2.id())),
      ]
      .into_iter()
      .collect()
    );
  }

  #[gpui_kit::test]
  fn insert_step_makes_it_a_suite_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step_a1 = case(&tests, p.case_a, cx).update(cx, |case, cx| case.remove_step(&p.step_a1, cx).unwrap());
    let updated = suite(&tests, p.suite_b, cx)
      .update(cx, |suite, cx| {
        suite.insert_child(TestSuiteChild::Step(step_a1), &p.suite_b, cx)
      })
      .unwrap();
    let new_path = TestPath::SuiteStep(p.suite_b.id(), p.step_a1.id());
    assert_eq!(updated, [(p.step_a1, new_path)].into_iter().collect());
    assert_eq!(child_names(&tests, p.suite_b, cx), vec!["Case B", "Suite step B", "Step A1"]);
  }

  #[gpui_kit::test]
  fn insert_child_out_of_the_suite_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = TestSuiteChild::Case(case(&tests, p.case_a, cx));
    let result = suite(&tests, p.suite_b, cx).update(cx, |suite, cx| suite.insert_child(case_a, &p.case_a, cx));
    assert!(matches!(result, Err(ProjectError::NoOp)));
    assert_eq!(child_names(&tests, p.suite_b, cx), vec!["Case B", "Suite step B"]);
  }

  // Lookups

  #[gpui_kit::test]
  fn step_from_path_finds_suite_steps_and_steps_of_cases(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite_a = suite(&tests, p.suite_a, cx);
    cx.read(|cx| {
      let suite_a = suite_a.read(cx);
      let step_path = |path: &TestPath| suite_a.step_from_path(path, cx).map(|step| step.read(cx).path());
      assert_eq!(step_path(&p.suite_step_a), Some(p.suite_step_a));
      assert_eq!(step_path(&p.step_a2), Some(p.step_a2));
      assert_eq!(step_path(&p.case_a), None);
      assert_eq!(step_path(&TestPath::SuiteStep(p.suite_a.id(), p.step_a1.id())), None);
      assert!(suite_a.case_from_path(&p.suite_step_a, cx).is_none());
    });
  }

  // Pasting

  fn copied_case(p: &Paths) -> ClipboardString {
    let FileSuiteChild::Case(case) = &p.file().suites[0].children[0] else {
      panic!("not a case");
    };
    clipboard(serde_json::to_string(case).unwrap(), CASE)
  }

  fn copied_step(p: &Paths) -> ClipboardString {
    clipboard(
      serde_json::to_string(&file_delay_step(p.step_a2.id(), "Step A2", 200)).unwrap(),
      STEP,
    )
  }

  #[gpui_kit::test]
  fn paste_case_on_the_suite_appends_a_copy(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    suite(&tests, p.suite_b, cx).update(cx, |suite, cx| suite.paste_from_clipboard(&copied_case(&p), p.suite_b, cx));
    assert_eq!(child_names(&tests, p.suite_b, cx), vec!["Case B", "Suite step B", "Case A"]);
    let pasted = subtree(&tests, p.suite_b, cx)[4..].to_vec();
    assert_eq!(pasted.len(), 3);
    for path in &pasted {
      assert!(pasted[0].is_parent(path));
      assert!(
        ![p.case_a, p.step_a1, p.step_a2]
          .iter()
          .any(|original| original.id() == path.id())
      );
    }
    assert_eq!(step_names(&tests, pasted[0], cx), vec!["Step A1", "Step A2"]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(pasted[0])]);
  }

  #[gpui_kit::test]
  fn paste_step_as_a_suite_step_after_the_child_of_target(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    suite(&tests, p.suite_b, cx).update(cx, |suite, cx| suite.paste_from_clipboard(&copied_step(&p), p.step_b1, cx));
    assert_eq!(child_names(&tests, p.suite_b, cx), vec!["Case B", "Step A2", "Suite step B"]);
    let pasted = subtree(&tests, p.suite_b, cx)[3];
    assert!(matches!(pasted, TestPath::SuiteStep(suite, step) if suite == p.suite_b.id() && step != p.step_a2.id()));
    assert!(step(&tests, pasted, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(200))));
  }

  #[gpui_kit::test]
  fn paste_invalid_content_is_ignored(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite_b = suite(&tests, p.suite_b, cx);
    for cb in [
      // Wrong kind, invalid json, json of another kind
      clipboard(copied_case(&p).text, SUITE),
      clipboard("not json".to_string(), CASE),
      clipboard(serde_json::to_string(&p.file().suites[0]).unwrap(), STEP),
      clipboard(copied_step(&p).text, CASE),
    ] {
      suite_b.update(cx, |suite, cx| suite.paste_from_clipboard(&cb, p.suite_b, cx));
    }
    // Unknown target
    suite_b.update(cx, |suite, cx| suite.paste_from_clipboard(&copied_case(&p), p.case_a, cx));
    assert_eq!(child_names(&tests, p.suite_b, cx), vec!["Case B", "Suite step B"]);
    assert!(events.borrow().is_empty());
  }

  // Metadata

  #[gpui_kit::test]
  fn metadata_setters_notify_changes_only(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    suite.update(cx, |suite, cx| {
      suite.set_name("Suite A".into(), cx);
      suite.set_description(None, cx);
    });
    assert!(events.borrow().is_empty());
    suite.update(cx, |suite, cx| {
      suite.set_name("Renamed".into(), cx);
      suite.set_description(Some("desc".into()), cx);
      suite.switch_disabled(cx);
    });
    suite.read_with(cx, |suite, _| {
      assert_eq!(suite.name(), "Renamed");
      assert_eq!(suite.description().as_deref(), Some("desc"));
      assert!(suite.disabled());
    });
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 3]);
  }
}
