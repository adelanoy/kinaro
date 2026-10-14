use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestMetadata, TestMetadataOwner, TestPath, TestsContainer, TestsContainerEvent, test_step::TestStep};
use gpui_kit::component::Icon;
use gpui_kit::{App, AppContext, ClipboardString, Context, Entity, SharedString, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileTestCase, FileTestStep};
use ki_utils::{Offset, STEP};
use log::{debug, warn};
use std::cmp::min;
use std::collections::HashMap;
use uuid::Uuid;

/// A test case: a named list of steps, located in a suite.
///
/// Cases are created with [`TestCase::new`], or loaded from their on-disk representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
  /// The container holding this case, notified of the changes made to the case
  pub(crate) container: WeakEntity<TestsContainer>,
  /// Metadata for the case itself (id, name, description, disabled flag).
  ///
  /// `meta.path` is always a [`TestPath::Case`].
  pub meta: TestMetadata,
  /// The case's steps, in display order
  steps: Vec<Entity<TestStep>>,
}

impl TestCase {
  /// Adds a new delay step of 1000 ms relative to `path`: right after the step at `path`, or
  /// appended at the end when `path` is the case itself or no step matches. The new step gets a
  /// fresh id and a unique name. Returns its path.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the new step.
  pub fn add_delay_step(&mut self, path: TestPath, cx: &mut Context<Self>) -> TestPath {
    let position = self
      .steps
      .iter()
      .position(|step| step.read(cx).meta.path.is_parent(&path))
      .map(|pos| min(pos + 1, self.steps.len()))
      .unwrap_or(self.steps.len());
    let name = ki_utils::next_available_name("new Delay", self.steps.iter().map(|step| step.read(cx).meta.name.clone()));
    let new_step = TestStep::new_delay(&self.meta.path, name, self.container.clone());
    let path = new_step.meta.path;
    self.steps.insert(position, cx.new(|_| new_step));
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(path)));
    path
  }

  /// Returns the path of this case followed by the paths of its steps, in display order.
  pub fn all_paths(&self, cx: &App) -> Vec<TestPath> {
    let mut paths = Vec::new();
    paths.push(self.meta.path);
    paths.extend(self.steps.iter().map(|step| step.read(cx).path()));
    paths
  }

  /// Returns a deep copy of this case named `name`, placed in the suite `parent_id`. The copy and
  /// its steps get fresh ids, while the steps keep their names.
  pub(crate) fn duplicate(&self, name: SharedString, parent_id: Uuid, cx: &mut Context<Self>) -> Self {
    let path = TestPath::Case(parent_id, Uuid::new_v4());
    let steps = self
      .steps
      .iter()
      .map(|step| {
        let name = step.read(cx).meta.name.clone();
        cx.new(|cx| step.update(cx, |step, _| step.duplicate(name, &path)))
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
      steps,
    }
  }

  /// Duplicates the step at `path` and inserts the copy right after it. The copy gets a fresh id
  /// and a unique name. Returns the path of the copy.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no step of this case is at `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the copy.
  pub fn duplicate_step(&mut self, path: &TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    let Some((ix, step)) = self
      .steps
      .iter()
      .enumerate()
      .find(|(_, step)| step.read(cx).meta.path() == *path)
    else {
      return Err(ProjectError::TestNotFound(*path));
    };
    let new_name = ki_utils::next_available_name(
      &step.read(cx).meta.name,
      self.steps.iter().map(|step| step.read(cx).meta.name.clone()),
    );
    let new_step = step.update(cx, |step, _| step.duplicate(new_name, &self.meta.path));
    let new_path = new_step.meta.path;
    self.steps.insert(min(ix + 1, self.steps.len()), cx.new(|_| new_step));
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
    Ok(new_path)
  }

  /// Builds a case of the suite `suite_id` from its on-disk representation. With `new_id`, the
  /// case and its steps get freshly generated ids (e.g. when pasted); otherwise ids are read from
  /// the file.
  pub fn from_file(
    file_test_case: FileTestCase,
    suite_id: Uuid,
    new_id: bool,
    container: WeakEntity<TestsContainer>,
    cx: &mut Context<Self>,
  ) -> Self {
    let meta = if new_id {
      TestMetadata::new_case_from_file(file_test_case.info, suite_id)
    } else {
      TestMetadata::case_from_file(file_test_case.info, suite_id)
    };
    let path = meta.path;
    Self {
      container: container.clone(),
      meta,
      steps: file_test_case
        .steps
        .into_iter()
        .map(|step| cx.new(|_| TestStep::from_file(step, &path, container.clone(), new_id)))
        .collect(),
    }
  }

  /// Produces the serializable, on-disk representation of this case.
  pub fn get_file(&self, cx: &App) -> FileTestCase {
    FileTestCase {
      info: self.meta.to_file(),
      steps: self.steps.iter().map(|step| step.read(cx).get_file()).collect(),
    }
  }

  /// Inserts `new_step` relative to `path`: appended at the end when `path` is a case path,
  /// otherwise right after the step at `path` (or at the end when none matches). The step is
  /// reparented under this case, keeping its id. Returns its new path.
  pub(crate) fn insert_step(&mut self, new_step: Entity<TestStep>, path: &TestPath, cx: &mut Context<Self>) -> TestPath {
    let (_, new_path) = new_step.update(cx, |step, _| step.reparent(&self.meta.path, true));
    let ix = if path.is_case() {
      self.steps.len()
    } else {
      self
        .steps
        .iter()
        .position(|step| step.read(cx).path() == *path)
        .map(|ix| min(ix + 1, self.steps.len()))
        .unwrap_or(self.steps.len())
    };
    self.steps.insert(ix, new_step);
    new_path
  }

  /// Creates a new, empty case of the suite `suite_id`, with a freshly generated id.
  pub fn new(suite_id: Uuid, name: SharedString, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      meta: TestMetadata {
        path: TestPath::Case(suite_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      steps: vec![],
    }
  }

  /// Moves the step at `path` one position down ([`Offset::Plus`]) or up ([`Offset::Minus`]). A
  /// no-op when it is already the last, or the first, step.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no step of this case is at `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the step moved.
  pub fn offset_step(&mut self, path: TestPath, offset: Offset, cx: &mut Context<Self>) -> ProjectResult<()> {
    let Some(pos) = self.steps.iter().position(|step| step.read(cx).meta.path == path) else {
      return Err(ProjectError::TestNotFound(path));
    };
    let new_pos = match offset {
      Offset::Plus => min(pos + 1, self.steps.len() - 1),
      Offset::Minus => pos.saturating_sub(1),
    };
    if pos != new_pos {
      self.steps.swap(pos, new_pos);
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    }
    Ok(())
  }

  /// Pastes the step held in `cb_str` (a serialized step, as told by its [`STEP`] metadata) as a
  /// step of this case relative to `target`: appended at the end when `target` is the case itself,
  /// otherwise right after the step at `target`. The pasted step gets a fresh id. Nothing is pasted
  /// if `target` isn't in this case (a warning is logged), or if the clipboard doesn't hold a valid
  /// step.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the pasted step.
  pub fn paste_from_clipboard(&mut self, cb_str: &ClipboardString, target: TestPath, cx: &mut Context<Self>) {
    // Pasted on the case itself: appended at the end, otherwise right after the targeted step
    let ix = if target == self.meta.path {
      Some(self.steps.len())
    } else {
      self
        .steps
        .iter()
        .position(|step| step.read(cx).meta.path == target)
        .map(|ix| min(ix + 1, self.steps.len()))
    };
    let Some(ix) = ix else {
      warn!("TestCase::paste_from_clipboard: Attempted to paste on a unknown path: {target}");
      return;
    };
    if cb_str.metadata.as_deref() != Some(STEP) {
      return;
    }
    let Ok(file_step) = serde_json::from_str::<FileTestStep>(cb_str.text()) else {
      warn!("Could not paste Step: Clipboard contains invalid Json");
      debug!("Invalid Json: {}", cb_str.text());
      return;
    };
    let step = TestStep::from_file(file_step, &self.meta.path, self.container.clone(), true);
    let new_path = step.meta.path;
    self.steps.insert(ix, cx.new(|_| step));
    _ = self
      .container
      .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
  }

  /// Removes the step at `path` and returns it, or `None` when no step of this case is at `path`.
  /// No event is emitted: the caller is responsible for notifying the removal.
  pub(crate) fn remove_step(&mut self, path: &TestPath, cx: &mut Context<Self>) -> Option<Entity<TestStep>> {
    let ix = self.steps.iter().position(|step| step.read(cx).path() == *path)?;
    Some(self.steps.remove(ix))
  }

  /// Moves this case, and its steps, under the suite `parent` goes through. With `keep_id` the ids
  /// are kept, otherwise fresh ones are generated. Returns the paths that changed, as
  /// `old path -> new path` (including unchanged paths, mapped to themselves).
  pub(crate) fn reparent(&mut self, parent: &TestPath, keep_id: bool, cx: &mut App) -> HashMap<TestPath, TestPath> {
    let id = if keep_id { self.meta.id() } else { Uuid::new_v4() };
    let new_path = TestPath::Case(parent.suite_id(), id);
    let mut path_updates = HashMap::new();
    path_updates.insert(self.path(), new_path);
    for step in &self.steps {
      let (old, new) = step.update(cx, |step, _| step.reparent(&new_path, keep_id));
      path_updates.insert(old, new);
    }
    self.meta.path = new_path;
    path_updates
  }

  /// Returns the step of this case addressed by `path`, or `None` when no step matches.
  pub(crate) fn step_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestStep>> {
    self.steps.iter().find(|step| step.read(cx).meta.path == *path).cloned()
  }

  /// Returns the steps of this case, in display order.
  #[inline]
  pub fn steps(&self) -> &[Entity<TestStep>] {
    &self.steps
  }
}

impl TestMetadataOwner for TestCase {
  #[inline]
  fn id(&self) -> Uuid {
    self.meta.id()
  }

  #[inline]
  fn path(&self) -> TestPath {
    self.meta.path
  }

  fn icon(&self) ->  impl Into<Icon> {
    IconAsset::TestCase
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
  use ki_project::FileSuiteChild;
  use ki_utils::CASE;

  /// Returns the on-disk representation of the case `ix` of the suite `suite_ix` of the reference tree.
  fn file_case_at(p: &Paths, suite_ix: usize, ix: usize) -> FileTestCase {
    let FileSuiteChild::Case(case) = p.file().suites[suite_ix].children[ix].clone() else {
      panic!("not a case");
    };
    case
  }

  #[gpui_kit::test]
  fn new_cases_are_empty_with_fresh_ids(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let suite_id = Uuid::new_v4();
    let case = TestCase::new(suite_id, "Case".into(), tests.downgrade());
    assert!(matches!(case.path(), TestPath::Case(suite, _) if suite == suite_id));
    assert!(case.steps().is_empty());
    assert_ne!(TestCase::new(suite_id, "Case".into(), tests.downgrade()).id(), case.id());
  }

  #[gpui_kit::test]
  fn all_paths_lists_the_case_then_its_steps(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    assert_eq!(subtree(&tests, p.case_a, cx), vec![p.case_a, p.step_a1, p.step_a2]);
  }

  #[gpui_kit::test]
  fn from_file_with_new_id_regenerates_every_id(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite_id = Uuid::new_v4();
    let file = file_case_at(&p, 0, 0);
    let weak = tests.downgrade();
    let kept = cx.new(|cx| TestCase::from_file(file.clone(), suite_id, false, weak.clone(), cx));
    let renewed = cx.new(|cx| TestCase::from_file(file, suite_id, true, weak, cx));
    cx.read(|cx| {
      let kept = kept.read(cx).all_paths(cx);
      assert_eq!(
        kept,
        vec![
          TestPath::Case(suite_id, p.case_a.id()),
          TestPath::Step(suite_id, p.case_a.id(), p.step_a1.id()),
          TestPath::Step(suite_id, p.case_a.id(), p.step_a2.id()),
        ]
      );
      let renewed = renewed.read(cx).all_paths(cx);
      assert_eq!(renewed.len(), 3);
      for path in &renewed {
        assert_eq!(path.suite_id(), suite_id);
        assert!(renewed[0].is_parent(path));
        assert!(kept.iter().all(|kept| kept.id() != path.id()));
      }
    });
  }

  #[gpui_kit::test]
  fn get_file_round_trips(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = case(&tests, p.case_a, cx);
    let saved = cx.read(|cx| case_a.read(cx).get_file(cx));
    assert_eq!(
      serde_json::to_value(saved).unwrap(),
      serde_json::to_value(file_case_at(&p, 0, 0)).unwrap()
    );
  }

  // Adding steps

  #[gpui_kit::test]
  fn add_delay_step_on_the_case_appends(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let new_path = case(&tests, p.case_a, cx).update(cx, |case, cx| case.add_delay_step(p.case_a, cx));
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2", "new Delay"]);
    assert!(p.case_a.is_parent(&new_path));
    assert!(step(&tests, new_path, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(1000))));
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(new_path)]);
  }

  #[gpui_kit::test]
  fn add_delay_step_inserts_after_the_step_of_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = case(&tests, p.case_a, cx);
    case_a.update(cx, |case, cx| case.add_delay_step(p.step_a1, cx));
    case_a.update(cx, |case, cx| case.add_delay_step(p.step_a1, cx));
    assert_eq!(
      step_names(&tests, p.case_a, cx),
      vec!["Step A1", "new Delay_1", "new Delay", "Step A2"]
    );
  }

  // Duplicating steps

  #[gpui_kit::test]
  fn duplicate_step_inserts_a_copy_after_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let copy = case(&tests, p.case_a, cx)
      .update(cx, |case, cx| case.duplicate_step(&p.step_a1, cx))
      .unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A1_1", "Step A2"]);
    assert!(p.case_a.is_parent(&copy));
    assert_ne!(copy, p.step_a1);
    assert!(step(&tests, copy, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(100))));
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(copy)]);
  }

  #[gpui_kit::test]
  fn duplicate_step_unknown_is_not_found(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let unknown = TestPath::Step(p.suite_a.id(), p.case_a.id(), Uuid::new_v4());
    let result = case(&tests, p.case_a, cx).update(cx, |case, cx| case.duplicate_step(&unknown, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == unknown));
  }

  #[gpui_kit::test]
  fn duplicate_keeps_the_steps_names_with_fresh_ids(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite_id = Uuid::new_v4();
    let copy = case(&tests, p.case_a, cx).update(cx, |case, cx| case.duplicate("Copy".into(), suite_id, cx));
    assert_eq!(copy.name(), "Copy");
    assert_eq!(copy.path().suite_id(), suite_id);
    cx.read(|cx| {
      let steps: Vec<_> = copy
        .steps()
        .iter()
        .map(|step| (step.read(cx).path(), step.read(cx).name()))
        .collect();
      assert_eq!(
        steps.iter().map(|(_, name)| name.as_ref()).collect::<Vec<_>>(),
        vec!["Step A1", "Step A2"]
      );
      for (path, _) in steps {
        assert!(copy.path().is_parent(&path));
        assert!(path != p.step_a1 && path != p.step_a2);
      }
    });
  }

  // Offsetting steps

  #[gpui_kit::test]
  fn offset_step_swaps_with_its_neighbour(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let case_a = case(&tests, p.case_a, cx);
    case_a
      .update(cx, |case, cx| case.offset_step(p.step_a1, Offset::Plus, cx))
      .unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A2", "Step A1"]);
    case_a
      .update(cx, |case, cx| case.offset_step(p.step_a1, Offset::Minus, cx))
      .unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2"]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 2]);
  }

  #[gpui_kit::test]
  fn offset_step_at_the_bounds_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let case_a = case(&tests, p.case_a, cx);
    case_a
      .update(cx, |case, cx| case.offset_step(p.step_a1, Offset::Minus, cx))
      .unwrap();
    case_a
      .update(cx, |case, cx| case.offset_step(p.step_a2, Offset::Plus, cx))
      .unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2"]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn offset_step_unknown_is_not_found(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let result = case(&tests, p.case_a, cx).update(cx, |case, cx| case.offset_step(p.step_b1, Offset::Plus, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == p.step_b1));
  }

  // Removing and inserting steps

  #[gpui_kit::test]
  fn remove_step_returns_it_without_notifying(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let case_a = case(&tests, p.case_a, cx);
    let step_a1 = step(&tests, p.step_a1, cx);
    assert_eq!(case_a.update(cx, |case, cx| case.remove_step(&p.step_a1, cx)), Some(step_a1));
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A2"]);
    assert_eq!(case_a.update(cx, |case, cx| case.remove_step(&p.step_b1, cx)), None);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn insert_step_reparents_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step_b1 = case(&tests, p.case_b, cx)
      .update(cx, |case, cx| case.remove_step(&p.step_b1, cx))
      .unwrap();
    let case_a = case(&tests, p.case_a, cx);
    let new_path = case_a.update(cx, |case, cx| case.insert_step(step_b1.clone(), &p.step_a1, cx));
    assert_eq!(new_path, TestPath::Step(p.suite_a.id(), p.case_a.id(), p.step_b1.id()));
    assert_eq!(step_b1.read_with(cx, |step, _| step.path()), new_path);
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step B1", "Step A2"]);
  }

  #[gpui_kit::test]
  fn reparent_reports_every_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = case(&tests, p.case_a, cx);
    let updates = case_a.update(cx, |case, cx| case.reparent(&p.suite_b, false, cx));
    let new_case = case_a.read_with(cx, |case, _| case.path());
    assert_eq!(new_case.suite_id(), p.suite_b.id());
    assert_ne!(new_case.id(), p.case_a.id());
    assert_eq!(updates.len(), 3);
    assert_eq!(updates.get(&p.case_a), Some(&new_case));
    for step in [p.step_a1, p.step_a2] {
      let new_step = updates[&step];
      assert!(new_case.is_parent(&new_step));
      assert_ne!(new_step.id(), step.id());
    }
  }

  // Pasting

  #[gpui_kit::test]
  fn paste_step_on_the_case_appends_a_copy(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let cb = clipboard(
      serde_json::to_string(&file_delay_step(p.step_a1.id(), "Step A1", 100)).unwrap(),
      STEP,
    );
    case(&tests, p.case_a, cx).update(cx, |case, cx| case.paste_from_clipboard(&cb, p.case_a, cx));
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2", "Step A1"]);
    let pasted = *subtree(&tests, p.case_a, cx).last().unwrap();
    assert_ne!(pasted, p.step_a1);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(pasted)]);
  }

  #[gpui_kit::test]
  fn paste_suite_step_after_the_target_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite_step = step(&tests, p.suite_step_a, cx).read_with(cx, |step, _| step.get_file());
    let cb = clipboard(serde_json::to_string(&suite_step).unwrap(), STEP);
    case(&tests, p.case_a, cx).update(cx, |case, cx| case.paste_from_clipboard(&cb, p.step_a1, cx));
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Suite step A", "Step A2"]);
    let pasted = subtree(&tests, p.case_a, cx)[2];
    assert_ne!(pasted.id(), p.suite_step_a.id());
    assert!(step(&tests, pasted, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(300))));
  }

  #[gpui_kit::test]
  fn paste_invalid_content_is_ignored(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let step = serde_json::to_string(&file_delay_step(Uuid::new_v4(), "Step", 1)).unwrap();
    let case_json = serde_json::to_string(&file_case_at(&p, 0, 0)).unwrap();
    let case_a = case(&tests, p.case_a, cx);
    for (cb, target) in [
      // A case can't be pasted in a case
      (clipboard(case_json.clone(), CASE), p.case_a),
      // A case passed off as a step
      (clipboard(case_json, STEP), p.case_a),
      (clipboard("not json".to_string(), STEP), p.case_a),
      // Unknown target
      (clipboard(step, STEP), p.step_b1),
    ] {
      case_a.update(cx, |case, cx| case.paste_from_clipboard(&cb, target, cx));
    }
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2"]);
    assert!(events.borrow().is_empty());
  }

  // Metadata

  #[gpui_kit::test]
  fn metadata_setters_notify_changes_only(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let case = case(&tests, p.case_a, cx);
    case.update(cx, |case, cx| {
      case.set_name("Case A".into(), cx);
      case.set_description(None, cx);
    });
    assert!(events.borrow().is_empty());
    case.update(cx, |case, cx| {
      case.set_name("Renamed".into(), cx);
      case.set_description(Some("desc".into()), cx);
      case.switch_disabled(cx);
      case.switch_disabled(cx);
    });
    case.read_with(cx, |case, _| {
      assert_eq!(case.name(), "Renamed");
      assert_eq!(case.description().as_deref(), Some("desc"));
      assert!(!case.disabled());
    });
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 4]);
  }
}
