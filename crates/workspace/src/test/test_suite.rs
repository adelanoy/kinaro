use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestMetadata, TestMetadataOwner, TestPath, TestsContainer, TestsContainerEvent, test_case::TestCase};
use gpui_kit::component::Icon;
use gpui_kit::{App, AppContext, ClipboardString, Context, Entity, SharedString, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileTestCase, FileTestStep, FileTestSuite};
use ki_utils::{CASE_MULTI, CASE_STEP, Offset, STEP};
use log::{debug, warn};
use std::cmp::min;
use std::collections::HashMap;
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
  pub cases: Vec<Entity<TestCase>>,
}

impl TestSuite {
  /// Adds a new case step (a delay of 1000 ms) relative to `path`: right after the case `path`
  /// points to or goes through, or appended at the end when `path` is the suite itself or no case
  /// matches. The new case step gets a fresh id and a unique name. Returns its path.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the new case step.
  pub fn add_new_delay_step(&mut self, path: TestPath, cx: &mut Context<Self>) -> TestPath {
    let position = self
      .cases
      .iter()
      .position(|case| case.read(cx).path().is_parent(&path))
      .map(|pos| min(pos + 1, self.cases.len()))
      .unwrap_or(self.cases.len());
    let name = ki_utils::next_available_name("new Case", self.cases.iter().map(|case| case.read(cx).meta.name.clone()));
    let new_case = TestCase::new_delay_case_step(self.meta.id(), name, self.container.clone());
    let path = new_case.meta.path;
    self.cases.insert(position, cx.new(|_| new_case));
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(path)));
    path
  }

  /// Adds a new, empty multi-step case relative to `path`: right after the case `path` points to
  /// or goes through, or appended at the end when `path` is the suite itself or no case matches.
  /// The new case gets a fresh id and a unique name. Returns its path.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the new case.
  pub fn add_new_test_case(&mut self, path: TestPath, cx: &mut Context<Self>) -> TestPath {
    let position = self
      .cases
      .iter()
      .position(|case| case.read(cx).meta.path.is_parent(&path))
      .map(|pos| min(pos + 1, self.cases.len()))
      .unwrap_or(self.cases.len());
    let name = ki_utils::next_available_name("new Case", self.cases.iter().map(|case| case.read(cx).meta.name.clone()));
    let new_case = TestCase::new_multi(self.meta.id(), name, self.container.clone());
    let path = new_case.meta.path;
    self.cases.insert(position, cx.new(|_| new_case));
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(path)));
    path
  }

  /// Returns the path of this suite followed by the paths of all of its descendants, in display order.
  pub fn all_paths(&self, cx: &App) -> Vec<TestPath> {
    let mut paths = Vec::new();
    paths.push(self.meta.path);
    paths.extend(
      self
        .cases
        .iter()
        .flat_map(|case| case.read_with(cx, |case, cx| case.all_paths(cx))),
    );
    paths
  }

  /// Returns the case of this suite that `path` points to or goes through
  /// (the owning case of a step), or `None` when no case matches.
  pub(crate) fn case_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestCase>> {
    self
      .cases
      .iter()
      .find(|case| case.read(cx).meta.path.is_parent(path))
      .cloned()
  }

  /// Returns a deep copy of this suite named `name`. The copy and all of its descendants get fresh
  /// ids, while the descendants keep their names.
  pub(crate) fn duplicate(&self, name: SharedString, cx: &mut Context<Self>) -> Self {
    let path = TestPath::Suite(Uuid::new_v4());
    let cases = self
      .cases
      .iter()
      .map(|case| {
        let name = case.read(cx).meta.name.clone();
        cx.new(|cx| case.update(cx, |case, cx| case.duplicate(name, path.suite_id(), cx)))
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
      cases,
    }
  }

  /// Duplicates the case at `path`, along with its steps, and inserts the copy right after it. The
  /// copy gets fresh ids and a unique name. Returns the path of the copy.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no case of this suite is at `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the copy.
  pub fn duplicate_case(&mut self, path: &TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    if let Some((ix, case)) = self
      .cases
      .iter()
      .enumerate()
      .find(|(_, case)| case.read(cx).meta.path() == *path)
    {
      let new_name = ki_utils::next_available_name(
        &case.read(cx).meta.name,
        self.cases.iter().map(|suite| suite.read(cx).meta.name.clone()),
      );
      let new_case = case.update(cx, |case, cx| case.duplicate(new_name, self.meta.id(), cx));
      let new_path = new_case.meta.path;
      self.cases.insert(min(ix + 1, self.cases.len()), cx.new(|_| new_case));
      _ = self
        .container
        .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
      Ok(new_path)
    } else {
      Err(ProjectError::TestNotFound(*path))
    }
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
      cases: file_test_suite
        .cases
        .into_iter()
        .map(|case| cx.new(|cx| TestCase::from_file(case, id, new_id, container.clone(), cx)))
        .collect(),
    }
  }

  /// Produces the serializable, on-disk representation of this suite.
  pub fn get_file(&self, cx: &App) -> FileTestSuite {
    FileTestSuite {
      info: self.meta.to_file(),
      cases: self.cases.iter().map(|case| case.read(cx).get_file(cx)).collect(),
    }
  }

  /// Inserts `case` relative to `path`: appended at the end when `path` is the suite itself,
  /// otherwise right after the case `path` points to or goes through (or at the end when none
  /// matches). The case, and its steps, are reparented under this suite, keeping their ids. Returns
  /// the paths that changed, as `old path -> new path`.
  ///
  /// # Errors
  /// [`ProjectError::NoOp`] if `path` isn't in this suite.
  pub(crate) fn insert_case(
    &mut self,
    case: Entity<TestCase>,
    path: &TestPath,
    cx: &mut Context<Self>,
  ) -> ProjectResult<HashMap<TestPath, TestPath>> {
    if self.meta.id() != path.suite_id() {
      return Err(ProjectError::NoOp);
    }
    let updated_paths = case.update(cx, |case, cx| case.reparent(&self.meta.path, true, cx));
    let ix = if matches!(path, TestPath::Suite(_)) {
      self.cases.len()
    } else {
      self
        .cases
        .iter()
        .position(|case| case.read(cx).path().is_parent(path))
        .map(|ix| min(ix + 1, self.cases.len()))
        .unwrap_or(self.cases.len())
    };
    self.cases.insert(ix, case);

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
      cases: vec![],
    }
  }

  /// Moves the case at `path` one position down ([`Offset::Plus`]) or up ([`Offset::Minus`]). A
  /// no-op when it is already the last, or the first, case.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no case of this suite is at `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the case moved.
  pub fn offset_case(&mut self, path: TestPath, offset: Offset, cx: &mut Context<Self>) -> ProjectResult<()> {
    if let Some(pos) = self.cases.iter().position(|case| case.read(cx).meta.path == path) {
      let new_pos = match offset {
        Offset::Plus => min(pos + 1, self.cases.len() - 1),
        Offset::Minus => {
          if pos > 0 {
            pos - 1
          } else {
            pos
          }
        }
      };
      if pos != new_pos {
        self.cases.swap(pos, new_pos);
        _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
      }
      Ok(())
    } else {
      Err(ProjectError::TestNotFound(path))
    }
  }

  /// Pastes the case or step held in `cb_str`, as told by its metadata (a serialized case, or a
  /// serialized step pasted as a case step), relative to `target`: appended at the end when `target`
  /// is the suite itself, otherwise right after the case `target` points to or goes through. The
  /// pasted node and its descendants get fresh ids. Nothing is pasted, and a warning is logged, if
  /// `target` isn't in this suite or the clipboard doesn't hold a valid case or step.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the pasted case.
  pub fn paste_from_clipboard(&mut self, cb_str: &ClipboardString, target: TestPath, cx: &mut Context<Self>) {
    // Pasted on the suite itself: appended at the end, otherwise right after the targeted case
    let ix = if target.is_suite() {
      Some(self.cases.len())
    } else {
      self
        .cases
        .iter()
        .position(|case| case.read(cx).meta.path.is_parent(&target))
        .map(|ix| min(ix + 1, self.cases.len()))
    };
    let Some(ix) = ix else {
      warn!("TestSuite::paste_from_clipboard: Attempted to paste on a unknown path: {target}");
      return;
    };
    if let Some(case) = if cb_str.metadata.as_deref() == Some(CASE_MULTI) || cb_str.metadata.as_deref() == Some(CASE_STEP) {
      self.paste_case(&cb_str.text, cx)
    } else if cb_str.metadata.as_deref() == Some(STEP) {
      self.paste_step(&cb_str.text, cx)
    } else {
      None
    } {
      let new_path = case.read(cx).meta.path;
      self.cases.insert(ix, case);
      _ = self
        .container
        .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
    }
  }

  /// Builds a case of this suite, with fresh ids, from `cb_content` (a serialized case), or returns
  /// `None` and logs a warning when it isn't one.
  fn paste_case(&mut self, cb_content: &str, cx: &mut Context<Self>) -> Option<Entity<TestCase>> {
    let Ok(case) = serde_json::from_str::<FileTestCase>(cb_content) else {
      warn!("Could not paste Case: Clipboard contains invalid Json");
      debug!("Invalid Json: {}", cb_content);
      return None;
    };
    Some(cx.new(|cx| TestCase::from_file(case, self.meta.id(), true, self.container.clone(), cx)))
  }

  /// Builds a case step of this suite, with a fresh id, from `cb_content` (a serialized step), or
  /// returns `None` and logs a warning when it isn't one.
  fn paste_step(&mut self, cb_content: &str, cx: &mut Context<Self>) -> Option<Entity<TestCase>> {
    let Ok(step) = serde_json::from_str::<FileTestStep>(cb_content) else {
      warn!("Could not paste Step: Clipboard contains invalid Json");
      debug!("Invalid Json: {}", cb_content);
      return None;
    };
    Some(cx.new(|_| TestCase::from_step_file(step, self.meta.id(), self.container.clone())))
  }

  /// Removes the case at `path` and returns it, or `None` when no case of this suite is at `path`.
  /// No event is emitted: the caller is responsible for notifying the removal.
  pub fn remove_case(&mut self, path: &TestPath, cx: &mut Context<Self>) -> Option<Entity<TestCase>> {
    let ix = self.cases.iter().position(|case| case.read(cx).path() == *path)?;
    let case = self.cases.remove(ix);
    Some(case)
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

  fn icon(&self) -> Icon {
    IconAsset::TestSuite.into()
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
  use crate::test::test_case::TestCaseType;
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
    assert!(suite.cases.is_empty());
    assert_ne!(TestSuite::new("Suite".into(), tests.downgrade()).id(), suite.id());
  }

  #[gpui_kit::test]
  fn all_paths_lists_the_suite_then_its_descendants(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    assert_eq!(
      subtree(&tests, p.suite_a, cx),
      vec![p.suite_a, p.case_a, p.step_a1, p.step_a2, p.case_step_a]
    );
  }

  // Adding cases

  #[gpui_kit::test]
  fn add_new_test_case_on_the_suite_appends(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let new_path = suite(&tests, p.suite_a, cx).update(cx, |suite, cx| suite.add_new_test_case(p.suite_a, cx));
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case A", "Case step A", "new Case"]);
    assert!(p.suite_a.is_parent(&new_path));
    case(&tests, new_path, cx).read_with(cx, |case, _| {
      assert!(matches!(case.case_type(), TestCaseType::CaseMulti { steps } if steps.is_empty()));
    });
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(new_path)]);
  }

  #[gpui_kit::test]
  fn add_new_test_case_inserts_after_the_case_of_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite = suite(&tests, p.suite_a, cx);
    suite.update(cx, |suite, cx| suite.add_new_test_case(p.step_a2, cx));
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case A", "new Case", "Case step A"]);
    suite.update(cx, |suite, cx| suite.add_new_test_case(p.case_step_a, cx));
    assert_eq!(
      case_names(&tests, p.suite_a, cx),
      vec!["Case A", "new Case", "Case step A", "new Case_1"]
    );
  }

  #[gpui_kit::test]
  fn add_new_delay_step_adds_a_case_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    let first = suite.update(cx, |suite, cx| suite.add_new_delay_step(p.case_a, cx));
    let second = suite.update(cx, |suite, cx| suite.add_new_delay_step(p.suite_a, cx));
    assert_eq!(
      case_names(&tests, p.suite_a, cx),
      vec!["Case A", "new Case", "Case step A", "new Case_1"]
    );
    case(&tests, first, cx).read_with(cx, |case, _| {
      assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(1000))));
    });
    assert_eq!(
      *events.borrow(),
      vec![TestsContainerEvent::TestAdded(first), TestsContainerEvent::TestAdded(second)]
    );
  }

  // Duplicating cases

  #[gpui_kit::test]
  fn duplicate_case_inserts_a_copy_after_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let copy = suite(&tests, p.suite_a, cx)
      .update(cx, |suite, cx| suite.duplicate_case(&p.case_a, cx))
      .unwrap();
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case A", "Case A_1", "Case step A"]);
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
  fn duplicate_case_step_keeps_its_data(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let copy = suite(&tests, p.suite_b, cx)
      .update(cx, |suite, cx| suite.duplicate_case(&p.case_step_b, cx))
      .unwrap();
    assert_eq!(
      case_names(&tests, p.suite_b, cx),
      vec!["Case B", "Case step B", "Case step B_1"]
    );
    case(&tests, copy, cx).read_with(cx, |case, _| {
      assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(500))));
    });
  }

  #[gpui_kit::test]
  fn duplicate_case_requires_a_case_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite = suite(&tests, p.suite_a, cx);
    for path in [p.step_a1, p.suite_a, TestPath::Case(p.suite_a.id(), Uuid::new_v4())] {
      let result = suite.update(cx, |suite, cx| suite.duplicate_case(&path, cx));
      assert!(matches!(result, Err(ProjectError::TestNotFound(not_found)) if not_found == path));
    }
  }

  // Offsetting cases

  #[gpui_kit::test]
  fn offset_case_swaps_with_its_neighbour(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    suite
      .update(cx, |suite, cx| suite.offset_case(p.case_a, Offset::Plus, cx))
      .unwrap();
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case step A", "Case A"]);
    suite
      .update(cx, |suite, cx| suite.offset_case(p.case_a, Offset::Minus, cx))
      .unwrap();
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case A", "Case step A"]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 2]);
  }

  #[gpui_kit::test]
  fn offset_case_at_the_bounds_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    suite
      .update(cx, |suite, cx| suite.offset_case(p.case_a, Offset::Minus, cx))
      .unwrap();
    suite
      .update(cx, |suite, cx| suite.offset_case(p.case_step_a, Offset::Plus, cx))
      .unwrap();
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case A", "Case step A"]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn offset_case_unknown_is_not_found(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let result = suite(&tests, p.suite_a, cx).update(cx, |suite, cx| suite.offset_case(p.case_b, Offset::Plus, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == p.case_b));
  }

  // Removing cases

  #[gpui_kit::test]
  fn remove_case_returns_it_without_notifying(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite = suite(&tests, p.suite_a, cx);
    let case_a = case(&tests, p.case_a, cx);
    assert_eq!(suite.update(cx, |suite, cx| suite.remove_case(&p.case_a, cx)), Some(case_a));
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case step A"]);
    assert_eq!(suite.update(cx, |suite, cx| suite.remove_case(&p.case_a, cx)), None);
    assert!(events.borrow().is_empty());
  }

  // Inserting cases

  #[gpui_kit::test]
  fn insert_case_reparents_it_and_its_steps(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = suite(&tests, p.suite_a, cx).update(cx, |suite, cx| suite.remove_case(&p.case_a, cx).unwrap());
    let suite_b = suite(&tests, p.suite_b, cx);
    let updated = suite_b
      .update(cx, |suite, cx| suite.insert_case(case_a, &p.case_b, cx))
      .unwrap();
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Case A", "Case step B"]);
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
  fn insert_case_out_of_the_suite_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = case(&tests, p.case_a, cx);
    let result = suite(&tests, p.suite_b, cx).update(cx, |suite, cx| suite.insert_case(case_a, &p.case_a, cx));
    assert!(matches!(result, Err(ProjectError::NoOp)));
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Case step B"]);
  }

  // Pasting

  fn copied_case(p: &Paths, ix: usize, metadata: &str) -> ClipboardString {
    clipboard(serde_json::to_string(&p.file().suites[0].cases[ix]).unwrap(), metadata)
  }

  #[gpui_kit::test]
  fn paste_multi_case_on_the_suite_appends_a_copy(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    suite(&tests, p.suite_b, cx).update(cx, |suite, cx| {
      suite.paste_from_clipboard(&copied_case(&p, 0, CASE_MULTI), p.suite_b, cx)
    });
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Case step B", "Case A"]);
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
  fn paste_case_step_after_the_case_of_target(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    suite(&tests, p.suite_b, cx).update(cx, |suite, cx| {
      suite.paste_from_clipboard(&copied_case(&p, 1, CASE_STEP), p.step_b1, cx)
    });
    assert_eq!(
      case_names(&tests, p.suite_b, cx),
      vec!["Case B", "Case step A", "Case step B"]
    );
    let pasted = subtree(&tests, p.suite_b, cx)[3];
    assert_ne!(pasted.id(), p.case_step_a.id());
    case(&tests, pasted, cx).read_with(cx, |case, _| {
      assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(300))));
    });
  }

  #[gpui_kit::test]
  fn paste_step_as_a_case_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step = clipboard(
      serde_json::to_string(&file_delay_step(p.step_a2.id(), "Step A2", 200)).unwrap(),
      STEP,
    );
    suite(&tests, p.suite_b, cx).update(cx, |suite, cx| suite.paste_from_clipboard(&step, p.case_b, cx));
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Step A2", "Case step B"]);
    let pasted = subtree(&tests, p.suite_b, cx)[3];
    assert_ne!(pasted.id(), p.step_a2.id());
    case(&tests, pasted, cx).read_with(cx, |case, _| {
      assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(200))));
    });
  }

  #[gpui_kit::test]
  fn paste_invalid_content_is_ignored(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let suite_b = suite(&tests, p.suite_b, cx);
    for cb in [
      // Wrong kind, invalid json, json of another kind, unknown target
      copied_case(&p, 0, SUITE),
      clipboard("not json".to_string(), CASE_MULTI),
      clipboard(serde_json::to_string(&p.file().suites[0]).unwrap(), STEP),
    ] {
      suite_b.update(cx, |suite, cx| suite.paste_from_clipboard(&cb, p.suite_b, cx));
    }
    suite_b.update(cx, |suite, cx| {
      suite.paste_from_clipboard(&copied_case(&p, 0, CASE_MULTI), p.case_a, cx)
    });
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Case step B"]);
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
