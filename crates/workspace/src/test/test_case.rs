use crate::error::{ProjectError, ProjectResult};
use crate::test::step_delay::DelayStep;
use crate::test::test_step::StepData;
use crate::test::{
  StepDataProvider, TestMetadata, TestMetadataOwner, TestPath, TestsContainer, TestsContainerEvent, test_step::TestStep,
};
use gpui_kit::component::Icon;
use gpui_kit::{App, AppContext, ClipboardString, Context, Entity, SharedString, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileTestCase, FileTestCaseType, FileTestStep};
use ki_utils::{CASE_STEP, Offset, STEP};
use log::{debug, warn};
use std::cmp::min;
use std::collections::HashMap;
use uuid::Uuid;

/// The two shapes a [`TestCase`] can take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestCaseType {
  /// A case that owns its own list of steps.
  CaseMulti { steps: Vec<Entity<TestStep>> },
  /// A case level step, so that it can be positioned directly under a suite
  CaseStep { data: StepData },
}

/// A test case: either a [`TestCaseType::CaseMulti`] holding its own steps,
/// or a [`TestCaseType::CaseStep`] (a case level step).
///
/// Cases are created with [`TestCase::new_multi`] / [`TestCase::new_delay_case_step`], or
/// loaded from their on-disk representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
  /// The container holding this case, notified of the changes made to the case
  pub(crate) container: WeakEntity<TestsContainer>,
  /// Metadata for the case itself (id, name, description, disabled flag).
  pub meta: TestMetadata,
  /// Whether the case holds its own steps or is a case step
  pub(crate) case_type: TestCaseType,
}

impl TestCase {
  /// Adds a new delay step of 1000 ms relative to `path`: right after the step at `path`, or
  /// appended at the end when `path` is the case itself or no step matches. The new step gets a
  /// fresh id and a unique name. Returns its path.
  ///
  /// # Errors
  /// [`ProjectError::OperationNotAllowed`] if this case is a [`TestCaseType::CaseStep`].
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the new step.
  pub fn add_delay_step(&mut self, path: TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        let position = steps
          .iter()
          .position(|step| step.read(cx).meta.path.is_parent(&path))
          .map(|pos| min(pos + 1, steps.len()))
          .unwrap_or(steps.len());
        let name = ki_utils::next_available_name("new Delay", steps.iter().map(|step| step.read(cx).meta.name.clone()));
        let new_case = TestStep::new_delay(self.meta.path().suite_id(), self.meta.id(), name, self.container.clone());
        let path = new_case.meta.path;
        steps.insert(position, cx.new(|_| new_case));
        _ = self
          .container
          .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(path)));
        Ok(path)
      }
      TestCaseType::CaseStep { .. } => Err(ProjectError::OperationNotAllowed),
    }
  }

  /// Returns the path of this case followed by the paths of its steps, in display order.
  pub fn all_paths(&self, cx: &App) -> Vec<TestPath> {
    let mut paths = Vec::new();
    paths.push(self.meta.path);
    if let TestCaseType::CaseMulti { steps } = &self.case_type {
      paths.extend(steps.iter().map(|step| step.read(cx).path()));
    }
    paths
  }

  /// Returns the [`TestCaseType`] variant backing this case.
  #[inline]
  pub fn case_type(&self) -> &TestCaseType {
    &self.case_type
  }

  /// Returns a deep copy of this case named `name`, placed in the suite `parent_id`. The copy and
  /// its steps get fresh ids, while the steps keep their names.
  pub(crate) fn duplicate(&self, name: SharedString, parent_id: Uuid, cx: &mut Context<Self>) -> Self {
    let path = TestPath::Case(parent_id, Uuid::new_v4());
    let meta = TestMetadata {
      path,
      name,
      description: self.meta.description.clone(),
      disabled: self.meta.disabled,
    };
    match &self.case_type {
      TestCaseType::CaseMulti { steps } => {
        let steps = steps
          .iter()
          .map(|step| {
            let name = step.read(cx).meta.name.clone();
            cx.new(|cx| step.update(cx, |step, _| step.duplicate(name, &path)))
          })
          .collect();
        Self {
          container: self.container.clone(),
          meta,
          case_type: TestCaseType::CaseMulti { steps },
        }
      }
      TestCaseType::CaseStep { data } => Self {
        container: self.container.clone(),
        meta,
        case_type: TestCaseType::CaseStep { data: data.clone() },
      },
    }
  }

  /// Duplicates the step at `path` and inserts the copy right after it. The copy gets a fresh id
  /// and a unique name. Returns the path of the copy.
  ///
  /// # Errors
  /// - [`ProjectError::TestNotFound`] if no step of this case is at `path`.
  /// - [`ProjectError::NoOp`] if this case is a [`TestCaseType::CaseStep`].
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the copy.
  pub fn duplicate_step(&mut self, path: &TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        if let Some((ix, step)) = steps.iter().enumerate().find(|(_, step)| step.read(cx).meta.path() == *path) {
          let new_name = ki_utils::next_available_name(
            &step.read(cx).meta.name,
            steps.iter().map(|suite| suite.read(cx).meta.name.clone()),
          );
          let new_step = step.update(cx, |step, _| step.duplicate(new_name, &self.meta.path));
          let new_path = new_step.meta.path;
          steps.insert(min(ix + 1, steps.len()), cx.new(|_| new_step));
          _ = self
            .container
            .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
          Ok(new_path)
        } else {
          Err(ProjectError::TestNotFound(*path))
        }
      }
      TestCaseType::CaseStep { .. } => Err(ProjectError::NoOp),
    }
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
    let info = if new_id {
      TestMetadata::new_case_from_file(file_test_case.info, suite_id)
    } else {
      TestMetadata::case_from_file(file_test_case.info, suite_id)
    };
    match file_test_case.case_type {
      FileTestCaseType::CaseMulti { steps } => {
        let path = info.path;
        Self {
          container: container.clone(),
          meta: info,
          case_type: TestCaseType::CaseMulti {
            steps: steps
              .into_iter()
              .map(|step| cx.new(|_| TestStep::from_file(step, path, container.clone(), new_id)))
              .collect(),
          },
        }
      }
      FileTestCaseType::CaseStep { data } => Self {
        container,
        meta: info,
        case_type: TestCaseType::CaseStep { data: data.into() },
      },
    }
  }

  /// Builds a case step of the suite `suite_id` from the on-disk representation of a step, with a
  /// freshly generated id.
  pub(crate) fn from_step_file(file_test_step: FileTestStep, suite_id: Uuid, container: WeakEntity<TestsContainer>) -> Self {
    let info = TestMetadata::new_case_from_file(file_test_step.info, suite_id);
    Self {
      container,
      meta: info,
      case_type: TestCaseType::CaseStep {
        data: file_test_step.data.into(),
      },
    }
  }

  /// Promotes a step to a case-level [`TestCaseType::CaseStep`], keeping
  /// the step's own id as the new case's id and reparenting it under
  /// `suite_id`.
  pub(crate) fn from_step(step: Entity<TestStep>, suite_id: Uuid, container: WeakEntity<TestsContainer>, cx: &App) -> Self {
    let (id, name, description, disabled, data) = step.read_with(cx, |step, _| {
      (step.id(), step.name(), step.description(), step.disabled(), step.data.clone())
    });
    let meta = TestMetadata {
      path: TestPath::Case(suite_id, id),
      name,
      description,
      disabled,
    };
    Self {
      container,
      meta,
      case_type: TestCaseType::CaseStep { data },
    }
  }

  /// Produces the serializable, on-disk representation of this case.
  pub fn get_file(&self, cx: &App) -> FileTestCase {
    FileTestCase {
      info: self.meta.to_file(),
      case_type: match &self.case_type {
        TestCaseType::CaseMulti { steps } => FileTestCaseType::CaseMulti {
          steps: steps.iter().map(|step| step.read(cx).get_file()).collect(),
        },
        TestCaseType::CaseStep { data } => FileTestCaseType::CaseStep { data: data.into() },
      },
    }
  }

  /// Inserts `new_step` relative to `path`: appended at the end when `path` is a case path,
  /// otherwise right after the step at `path` (or at the end when none matches). The step is
  /// reparented under this case, keeping its id. Returns its new path.
  ///
  /// # Errors
  /// [`ProjectError::NoOp`] if this case is a [`TestCaseType::CaseStep`].
  pub(crate) fn insert_step(
    &mut self,
    new_step: Entity<TestStep>,
    path: &TestPath,
    cx: &mut Context<Self>,
  ) -> ProjectResult<TestPath> {
    let (_, new_path) = new_step.update(cx, |step, _| step.reparent(&self.meta.path, true));
    if let TestCaseType::CaseMulti { steps } = &mut self.case_type {
      let ix = if path.is_case() {
        steps.len()
      } else {
        steps
          .iter()
          .position(|step| step.read(cx).path() == *path)
          .map(|ix| min(ix + 1, steps.len()))
          .unwrap_or(steps.len())
      };
      steps.insert(ix, new_step);
      Ok(new_path)
    } else {
      Err(ProjectError::NoOp)
    }
  }

  /// Returns `true` if this case is a [`TestCaseType::CaseStep`].
  #[inline]
  pub fn is_case_step(&self) -> bool {
    matches!(self.case_type, TestCaseType::CaseStep { .. })
  }

  /// Creates a new, empty [`TestCaseType::CaseMulti`] with a freshly
  /// generated id.
  pub fn new_multi(suite_id: Uuid, name: SharedString, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      meta: TestMetadata {
        path: TestPath::Case(suite_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      case_type: TestCaseType::CaseMulti { steps: vec![] },
    }
  }

  /// Creates a new [`TestCaseType::CaseStep`] holding a delay step of 1000 ms,
  /// with a freshly generated id.
  pub fn new_delay_case_step(suite_id: Uuid, name: SharedString, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      meta: TestMetadata {
        path: TestPath::Case(suite_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      case_type: TestCaseType::CaseStep {
        data: StepData::Delay(DelayStep(1000)),
      },
    }
  }

  /// Moves the step at `path` one position down ([`Offset::Plus`]) or up ([`Offset::Minus`]). A
  /// no-op when it is already the last, or the first, step.
  ///
  /// # Errors
  /// - [`ProjectError::TestNotFound`] if no step of this case is at `path`.
  /// - [`ProjectError::NoOp`] if this case is a [`TestCaseType::CaseStep`].
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the step moved.
  pub fn offset_step(&mut self, path: TestPath, offset: Offset, cx: &mut Context<Self>) -> ProjectResult<()> {
    match &mut self.case_type {
      TestCaseType::CaseMulti { steps } => {
        if let Some(pos) = steps.iter().position(|case| case.read(cx).meta.path == path) {
          let new_pos = match offset {
            Offset::Plus => min(pos + 1, steps.len() - 1),
            Offset::Minus => {
              if pos > 0 {
                pos - 1
              } else {
                pos
              }
            }
          };
          if pos != new_pos {
            steps.swap(pos, new_pos);
            _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
          }
          Ok(())
        } else {
          Err(ProjectError::TestNotFound(path))
        }
      }
      TestCaseType::CaseStep { .. } => Err(ProjectError::NoOp),
    }
  }

  /// Builds a step of the case at `path`, with a fresh id, from `cb_content` (a serialized case
  /// step), or returns `None` when it isn't one (a warning is logged when it isn't a case at all).
  fn paste_case_step(cb_content: &str, path: TestPath, container: WeakEntity<TestsContainer>) -> Option<TestStep> {
    let Ok(file_case) = serde_json::from_str::<FileTestCase>(cb_content) else {
      warn!("Could not paste Case: Clipboard contains invalid Json");
      debug!("Invalid Json: {}", cb_content);
      return None;
    };
    TestStep::from_case_file(file_case, path, container)
  }

  /// Pastes the step or case step held in `cb_str`, as told by its metadata, as a step of this case
  /// relative to `target`: appended at the end when `target` is the case itself, otherwise right
  /// after the step at `target`. The pasted step gets a fresh id. Nothing is pasted if this case is
  /// a [`TestCaseType::CaseStep`], if `target` isn't in this case (a warning is logged), or if the
  /// clipboard doesn't hold a valid step or case step.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] on the owning container, with the path of the pasted step.
  pub fn paste_from_clipboard(&mut self, cb_str: &ClipboardString, target: TestPath, cx: &mut Context<Self>) {
    if let TestCaseType::CaseMulti { steps } = &mut self.case_type {
      // Pasted on the case itself: appended at the end, otherwise right after the targeted step
      let ix = if target == self.meta.path {
        Some(steps.len())
      } else {
        steps
          .iter()
          .position(|step| step.read(cx).meta.path == target)
          .map(|ix| min(ix + 1, steps.len()))
      };
      let Some(ix) = ix else {
        warn!("TestCase::paste_from_clipboard: Attempted to paste on a unknown path: {target}");
        return;
      };
      if let Some(step) = if cb_str.metadata.as_deref() == Some(CASE_STEP) {
        Self::paste_case_step(cb_str.text(), self.meta.path, self.container.clone())
      } else if cb_str.metadata.as_deref() == Some(STEP) {
        Self::paste_step(cb_str.text(), self.meta.path, self.container.clone())
      } else {
        None
      } {
        let new_path = step.meta.path;
        steps.insert(ix, cx.new(|_| step));
        _ = self
          .container
          .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestAdded(new_path)));
      }
    }
  }

  /// Builds a step of the case at `path`, with a fresh id, from `cb_content` (a serialized step), or
  /// returns `None` and logs a warning when it isn't one.
  fn paste_step(cb_content: &str, path: TestPath, container: WeakEntity<TestsContainer>) -> Option<TestStep> {
    let Ok(file_step) = serde_json::from_str::<FileTestStep>(cb_content) else {
      warn!("Could not paste Case: Clipboard contains invalid Json");
      debug!("Invalid Json: {}", cb_content);
      return None;
    };
    Some(TestStep::from_file(file_step, path, container, true))
  }

  /// Removes the step at `path` and returns it, or `None` when this case is a
  /// [`TestCaseType::CaseStep`] or no step of it is at `path`. No event is emitted: the caller is
  /// responsible for notifying the removal.
  pub(crate) fn remove_step(&mut self, path: &TestPath, cx: &mut Context<Self>) -> Option<Entity<TestStep>> {
    if let TestCaseType::CaseMulti { steps } = &mut self.case_type {
      let ix = steps.iter().position(|case| case.read(cx).path() == *path)?;
      let step = steps.remove(ix);
      Some(step)
    } else {
      None
    }
  }

  /// Moves this case, and its steps, under the suite `parent` goes through. With `keep_id` the ids
  /// are kept, otherwise fresh ones are generated. Returns the paths that changed, as
  /// `old path -> new path` (including unchanged paths, mapped to themselves).
  pub(crate) fn reparent(&mut self, parent: &TestPath, keep_id: bool, cx: &mut App) -> HashMap<TestPath, TestPath> {
    let id = if keep_id { self.meta.id() } else { Uuid::new_v4() };
    let new_path = TestPath::Case(parent.suite_id(), id);
    let mut path_updates = HashMap::new();
    path_updates.insert(self.path(), new_path);
    if let TestCaseType::CaseMulti { steps } = &mut self.case_type {
      for step in steps {
        let (old, new) = step.update(cx, |step, _| step.reparent(&new_path, keep_id));
        path_updates.insert(old, new);
      }
    }
    self.meta.path = new_path;
    path_updates
  }

  /// Returns the step of this case addressed by `path`, or `None` when this
  /// is a [`TestCaseType::CaseStep`] or no step matches.
  pub(crate) fn step_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestStep>> {
    if let TestCaseType::CaseMulti { steps } = &self.case_type {
      steps.iter().find(|step| step.read(cx).meta.path == *path).cloned()
    } else {
      None
    }
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

  fn icon(&self) -> Icon {
    if let TestCaseType::CaseStep { data } = &self.case_type {
      data.icon()
    } else {
      IconAsset::TestCase.into()
    }
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

impl StepDataProvider for TestCase {
  fn data(&self) -> Option<&StepData> {
    if let TestCaseType::CaseStep { data } = &self.case_type {
      Some(data)
    } else {
      None
    }
  }

  fn data_mut(&mut self) -> Option<&mut StepData> {
    if let TestCaseType::CaseStep { data } = &mut self.case_type {
      Some(data)
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::test_fixtures::*;
  use gpui_kit::TestAppContext;
  use ki_utils::CASE_MULTI;

  #[gpui_kit::test]
  fn new_cases_have_fresh_ids(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let suite_id = Uuid::new_v4();
    let multi = TestCase::new_multi(suite_id, "Multi".into(), tests.downgrade());
    assert!(matches!(multi.path(), TestPath::Case(suite, _) if suite == suite_id));
    assert!(matches!(multi.case_type(), TestCaseType::CaseMulti { steps } if steps.is_empty()));
    assert!(!multi.is_case_step());
    let case_step = TestCase::new_delay_case_step(suite_id, "Step".into(), tests.downgrade());
    assert!(matches!(case_step.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(1000))));
    assert!(case_step.is_case_step());
    assert_ne!(multi.id(), case_step.id());
  }

  #[gpui_kit::test]
  fn all_paths_lists_the_case_then_its_steps(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    assert_eq!(subtree(&tests, p.case_a, cx), vec![p.case_a, p.step_a1, p.step_a2]);
    assert_eq!(subtree(&tests, p.case_step_a, cx), vec![p.case_step_a]);
  }

  #[gpui_kit::test]
  fn from_file_with_new_id_regenerates_every_id(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite_id = Uuid::new_v4();
    let file = p.file().suites[0].cases[0].clone();
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
    for (ix, path) in [p.case_a, p.case_step_a].into_iter().enumerate() {
      let case = case(&tests, path, cx);
      let saved = cx.read(|cx| case.read(cx).get_file(cx));
      assert_eq!(
        serde_json::to_value(saved).unwrap(),
        serde_json::to_value(&p.file().suites[0].cases[ix]).unwrap()
      );
    }
  }

  #[gpui_kit::test]
  fn from_step_promotes_a_step_keeping_its_id(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step_a2 = step(&tests, p.step_a2, cx);
    let case = cx.read(|cx| TestCase::from_step(step_a2, p.suite_b.id(), tests.downgrade(), cx));
    assert_eq!(case.path(), TestPath::Case(p.suite_b.id(), p.step_a2.id()));
    assert_eq!(case.name(), "Step A2");
    assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(200))));
  }

  #[gpui_kit::test]
  fn from_step_file_builds_a_case_step_with_a_fresh_id(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (suite_id, step_id) = (Uuid::new_v4(), Uuid::new_v4());
    let case = TestCase::from_step_file(file_delay_step(step_id, "Step", 42), suite_id, tests.downgrade());
    assert!(matches!(case.path(), TestPath::Case(suite, case) if suite == suite_id && case != step_id));
    assert_eq!(case.name(), "Step");
    assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(42))));
  }

  // Adding steps

  #[gpui_kit::test]
  fn add_delay_step_on_the_case_appends(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let new_path = case(&tests, p.case_a, cx)
      .update(cx, |case, cx| case.add_delay_step(p.case_a, cx))
      .unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2", "new Delay"]);
    assert!(p.case_a.is_parent(&new_path));
    assert!(step(&tests, new_path, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(1000))));
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(new_path)]);
  }

  #[gpui_kit::test]
  fn add_delay_step_inserts_after_the_step_of_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = case(&tests, p.case_a, cx);
    case_a.update(cx, |case, cx| case.add_delay_step(p.step_a1, cx)).unwrap();
    case_a.update(cx, |case, cx| case.add_delay_step(p.step_a1, cx)).unwrap();
    assert_eq!(
      step_names(&tests, p.case_a, cx),
      vec!["Step A1", "new Delay_1", "new Delay", "Step A2"]
    );
  }

  #[gpui_kit::test]
  fn add_delay_step_on_a_case_step_is_not_allowed(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let result = case(&tests, p.case_step_a, cx).update(cx, |case, cx| case.add_delay_step(p.case_step_a, cx));
    assert!(matches!(result, Err(ProjectError::OperationNotAllowed)));
    assert!(events.borrow().is_empty());
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
  fn duplicate_step_errors(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let unknown = TestPath::Step(p.suite_a.id(), p.case_a.id(), Uuid::new_v4());
    let result = case(&tests, p.case_a, cx).update(cx, |case, cx| case.duplicate_step(&unknown, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == unknown));
    let result = case(&tests, p.case_step_a, cx).update(cx, |case, cx| case.duplicate_step(&p.case_step_a, cx));
    assert!(matches!(result, Err(ProjectError::NoOp)));
  }

  #[gpui_kit::test]
  fn duplicate_keeps_the_steps_names_with_fresh_ids(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let suite_id = Uuid::new_v4();
    let copy = case(&tests, p.case_a, cx).update(cx, |case, cx| case.duplicate("Copy".into(), suite_id, cx));
    assert_eq!(copy.name(), "Copy");
    assert_eq!(copy.path().suite_id(), suite_id);
    let TestCaseType::CaseMulti { steps } = copy.case_type() else {
      panic!("the copy should be a multi-case");
    };
    cx.read(|cx| {
      let steps: Vec<_> = steps
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
  fn offset_step_errors(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let result = case(&tests, p.case_a, cx).update(cx, |case, cx| case.offset_step(p.step_b1, Offset::Plus, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == p.step_b1));
    let result = case(&tests, p.case_step_a, cx).update(cx, |case, cx| case.offset_step(p.case_step_a, Offset::Plus, cx));
    assert!(matches!(result, Err(ProjectError::NoOp)));
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
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn remove_step_unknown_or_on_a_case_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    assert_eq!(
      case(&tests, p.case_a, cx).update(cx, |case, cx| case.remove_step(&p.step_b1, cx)),
      None
    );
    assert_eq!(
      case(&tests, p.case_step_a, cx).update(cx, |case, cx| case.remove_step(&p.case_step_a, cx)),
      None
    );
  }

  #[gpui_kit::test]
  fn insert_step_reparents_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step_b1 = case(&tests, p.case_b, cx)
      .update(cx, |case, cx| case.remove_step(&p.step_b1, cx))
      .unwrap();
    let case_a = case(&tests, p.case_a, cx);
    let new_path = case_a
      .update(cx, |case, cx| case.insert_step(step_b1.clone(), &p.step_a1, cx))
      .unwrap();
    assert_eq!(new_path, TestPath::Step(p.suite_a.id(), p.case_a.id(), p.step_b1.id()));
    assert_eq!(step_b1.read_with(cx, |step, _| step.path()), new_path);
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step B1", "Step A2"]);
  }

  #[gpui_kit::test]
  fn insert_step_in_a_case_step_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step_b1 = step(&tests, p.step_b1, cx);
    let result = case(&tests, p.case_step_a, cx).update(cx, |case, cx| case.insert_step(step_b1, &p.case_step_a, cx));
    assert!(matches!(result, Err(ProjectError::NoOp)));
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
  fn paste_case_step_after_the_target_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let cb = clipboard(serde_json::to_string(&p.file().suites[0].cases[1]).unwrap(), CASE_STEP);
    case(&tests, p.case_a, cx).update(cx, |case, cx| case.paste_from_clipboard(&cb, p.step_a1, cx));
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Case step A", "Step A2"]);
    let pasted = subtree(&tests, p.case_a, cx)[2];
    assert_ne!(pasted.id(), p.case_step_a.id());
    assert!(step(&tests, pasted, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(300))));
  }

  #[gpui_kit::test]
  fn paste_invalid_content_is_ignored(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let step = serde_json::to_string(&file_delay_step(Uuid::new_v4(), "Step", 1)).unwrap();
    let multi_case = serde_json::to_string(&p.file().suites[0].cases[0]).unwrap();
    let case_a = case(&tests, p.case_a, cx);
    for (cb, target) in [
      // A multi-case can't be pasted in a case
      (clipboard(multi_case.clone(), CASE_MULTI), p.case_a),
      // A multi-case passed off as a case step
      (clipboard(multi_case, CASE_STEP), p.case_a),
      (clipboard("not json".to_string(), STEP), p.case_a),
      // Unknown target
      (clipboard(step.clone(), STEP), p.step_b1),
    ] {
      case_a.update(cx, |case, cx| case.paste_from_clipboard(&cb, target, cx));
    }
    // A case step has no steps
    case(&tests, p.case_step_a, cx).update(cx, |case, cx| {
      case.paste_from_clipboard(&clipboard(step, STEP), p.case_step_a, cx)
    });
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2"]);
    assert!(events.borrow().is_empty());
  }

  // Metadata

  #[gpui_kit::test]
  fn metadata_setters_notify_changes_only(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let case = case(&tests, p.case_step_a, cx);
    case.update(cx, |case, cx| {
      case.set_name("Case step A".into(), cx);
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
