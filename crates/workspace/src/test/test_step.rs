use crate::test::step_delay::DelayStep;
use crate::test::step_rest::RestStep;
use crate::test::{TestMetadata, TestMetadataOwner, TestPath, TestsContainer, TestsContainerEvent};
use gpui_kit::component::Icon;
use gpui_kit::{Context, SharedString, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileStepData, FileTestStep};
use uuid::Uuid;

/// What a step does when run, one variant per kind of step.
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum StepData {
  /// Sends a REST request
  RestStep(RestStep),
  /// Waits for a given time
  Delay(DelayStep),
}

impl StepData {
  pub fn icon(&self) ->  impl Into<Icon> {
    match self {
      StepData::RestStep(_) => IconAsset::StepRest,
      StepData::Delay(_) => IconAsset::StepDelay,
    }
  }
}

impl From<FileStepData> for StepData {
  fn from(value: FileStepData) -> Self {
    match value {
      FileStepData::RestStep(rest_step) => StepData::RestStep(rest_step.into()),
      FileStepData::Delay(delay_step) => StepData::Delay(delay_step.into()),
    }
  }
}

impl From<&StepData> for FileStepData {
  fn from(value: &StepData) -> Self {
    match value {
      StepData::RestStep(rest_step) => FileStepData::RestStep(rest_step.into()),
      StepData::Delay(delay_step) => FileStepData::Delay(delay_step.into()),
    }
  }
}

/// A single test step: either one of a [`TestCase`](crate::test::test_case::TestCase)'s
/// own steps, or a step located directly in a [`TestSuite`](crate::test::test_suite::TestSuite).
///
/// Wherever a step is placed, it is given a parent path: a suite or a suite step path places it
/// directly in that suite, a case or a step path places it in that case (see [`TestPath::new_step`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestStep {
  /// The container holding this step, notified of the changes made to the step
  container: WeakEntity<TestsContainer>,
  /// Metadata for the step itself (id, name, description, disabled flag).
  ///
  /// `meta.path` is always a [`TestPath::SuiteStep`] or a [`TestPath::Step`].
  pub meta: TestMetadata,
  /// What the step does when run
  pub(crate) data: StepData,
}

impl TestStep {
  /// Returns what the step does when run.
  #[inline]
  pub fn data(&self) -> &StepData {
    &self.data
  }

  /// Returns what the step does when run, for editing.
  #[inline]
  pub fn data_mut(&mut self) -> &mut StepData {
    &mut self.data
  }

  /// Returns a copy of this step named `name`, placed where `parent_path` points, with a fresh id.
  pub(crate) fn duplicate(&self, name: SharedString, parent_path: &TestPath) -> Self {
    let path = TestPath::new_step(parent_path, Uuid::new_v4());
    Self {
      container: self.container.clone(),
      meta: TestMetadata {
        path,
        name,
        description: self.meta.description.clone(),
        disabled: self.meta.disabled,
      },
      data: self.data.clone(),
    }
  }

  /// Builds a step placed where `parent_path` points from its on-disk representation. With
  /// `new_id`, the step gets a freshly generated id (e.g. when pasted); otherwise its id is read
  /// from the file.
  pub(crate) fn from_file(
    file_step: FileTestStep,
    parent_path: &TestPath,
    container: WeakEntity<TestsContainer>,
    new_id: bool,
  ) -> TestStep {
    Self {
      container,
      meta: if new_id {
        TestMetadata::new_step_from_file(file_step.info, parent_path)
      } else {
        TestMetadata::step_from_file(file_step.info, parent_path)
      },
      data: file_step.data.into(),
    }
  }

  /// Produces the serializable, on-disk representation of this step.
  pub fn get_file(&self) -> FileTestStep {
    FileTestStep {
      info: self.meta.to_file(),
      data: (&self.data).into(),
    }
  }

  /// Creates a new delay step of 1000 ms placed where `parent_path` points, with a freshly
  /// generated id.
  pub fn new_delay(parent_path: &TestPath, name: SharedString, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      meta: TestMetadata {
        path: TestPath::new_step(parent_path, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      data: StepData::Delay(DelayStep(1000)),
    }
  }

  /// Moves this step where `parent` points. With `keep_id` the id is kept, otherwise a fresh one is
  /// generated. Returns the path change, as `(old path, new path)`.
  pub(crate) fn reparent(&mut self, parent: &TestPath, keep_id: bool) -> (TestPath, TestPath) {
    let id = if keep_id { self.meta.id() } else { Uuid::new_v4() };
    let new_path = TestPath::new_step(parent, id);
    let path_update = (self.path(), new_path);
    self.meta.path = new_path;
    path_update
  }
}

impl TestMetadataOwner for TestStep {
  #[inline]
  fn id(&self) -> Uuid {
    self.meta.id()
  }

  #[inline]
  fn path(&self) -> TestPath {
    self.meta.path
  }

  fn icon(&self) ->  impl Into<Icon> {
    self.data.icon()
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
  use crate::test::step_rest::{RestBody, RestMethod};
  use crate::test::test_fixtures::*;
  use gpui_kit::TestAppContext;
  use ki_project::{FileRestBody, FileRestMethod, FileRestStep};

  #[gpui_kit::test]
  fn new_delay_has_a_fresh_id(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (suite_id, case_id) = (Uuid::new_v4(), Uuid::new_v4());
    let step = TestStep::new_delay(&TestPath::Case(suite_id, case_id), "Delay".into(), tests.downgrade());
    assert!(matches!(step.path(), TestPath::Step(suite, case, _) if suite == suite_id && case == case_id));
    assert_eq!(step.name(), "Delay");
    assert_eq!(step.data, StepData::Delay(DelayStep(1000)));
    let suite_step = TestStep::new_delay(&TestPath::Suite(suite_id), "Delay".into(), tests.downgrade());
    assert!(matches!(suite_step.path(), TestPath::SuiteStep(suite, _) if suite == suite_id));
    assert_ne!(suite_step.id(), step.id());
  }

  #[gpui_kit::test]
  fn from_file_keeps_or_renews_the_id(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let step_id = Uuid::new_v4();
    let file = file_delay_step(step_id, "Step", 10);
    for parent in [
      TestPath::Case(Uuid::new_v4(), Uuid::new_v4()),
      TestPath::Suite(Uuid::new_v4()),
    ] {
      let kept = TestStep::from_file(file.clone(), &parent, tests.downgrade(), false);
      assert_eq!(kept.path(), TestPath::new_step(&parent, step_id));
      assert_eq!(kept.data, StepData::Delay(DelayStep(10)));
      let renewed = TestStep::from_file(file.clone(), &parent, tests.downgrade(), true);
      assert!(parent.is_parent(&renewed.path()));
      assert_ne!(renewed.id(), step_id);
    }
  }

  #[gpui_kit::test]
  fn get_file_round_trips_a_rest_step(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let file = FileTestStep {
      info: file_meta(Uuid::new_v4(), "Rest"),
      data: FileStepData::RestStep(FileRestStep {
        method: FileRestMethod::Post,
        endpoint: "/items".to_string(),
        params: vec![],
        body: FileRestBody::Raw {
          content_type: "application/json".to_string(),
          lines: vec!["{".to_string(), "}".to_string()],
        },
      }),
    };
    let parent = TestPath::Case(Uuid::new_v4(), Uuid::new_v4());
    let step = TestStep::from_file(file.clone(), &parent, tests.downgrade(), false);
    let StepData::RestStep(rest) = &step.data else {
      panic!("should be a rest step");
    };
    assert_eq!(rest.method, RestMethod::Post);
    assert!(matches!(rest.body, RestBody::Raw { .. }));
    assert_eq!(
      serde_json::to_value(step.get_file()).unwrap(),
      serde_json::to_value(file).unwrap()
    );
  }

  #[gpui_kit::test]
  fn duplicate_gives_a_fresh_id_in_the_parent(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let copy = step(&tests, p.step_a1, cx).read_with(cx, |step, _| step.duplicate("Copy".into(), &p.case_b));
    assert_eq!(copy.name(), "Copy");
    assert!(p.case_b.is_parent(&copy.path()));
    assert_ne!(copy.id(), p.step_a1.id());
    assert_eq!(copy.data, StepData::Delay(DelayStep(100)));
    let copy = step(&tests, p.step_a1, cx).read_with(cx, |step, _| step.duplicate("Copy".into(), &p.suite_b));
    assert!(matches!(copy.path(), TestPath::SuiteStep(suite, _) if suite == p.suite_b.id()));
  }

  #[gpui_kit::test]
  fn reparent_keeps_or_renews_the_id(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step_a1 = step(&tests, p.step_a1, cx);
    let (old, new) = step_a1.update(cx, |step, _| step.reparent(&p.case_b, true));
    assert_eq!(
      (old, new),
      (p.step_a1, TestPath::Step(p.suite_b.id(), p.case_b.id(), p.step_a1.id()))
    );
    let (old, renewed) = step_a1.update(cx, |step, _| step.reparent(&p.step_b1, false));
    assert_eq!(old, new);
    assert!(p.case_b.is_parent(&renewed));
    assert_ne!(renewed.id(), p.step_a1.id());
    assert_eq!(step_a1.read_with(cx, |step, _| step.path()), renewed);
  }

  #[gpui_kit::test]
  fn reparent_in_a_suite_makes_a_suite_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let step_a1 = step(&tests, p.step_a1, cx);
    let (_, new) = step_a1.update(cx, |step, _| step.reparent(&p.suite_b, true));
    assert_eq!(new, TestPath::SuiteStep(p.suite_b.id(), p.step_a1.id()));
    let (_, new) = step_a1.update(cx, |step, _| step.reparent(&p.suite_step_a, true));
    assert_eq!(new, TestPath::SuiteStep(p.suite_a.id(), p.step_a1.id()));
    let (_, new) = step_a1.update(cx, |step, _| step.reparent(&p.case_a, true));
    assert_eq!(new, p.step_a1);
  }

  #[gpui_kit::test]
  fn metadata_setters_notify_changes_only(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let step = step(&tests, p.step_b1, cx);
    step.update(cx, |step, cx| {
      step.set_name("Step B1".into(), cx);
      step.set_description(None, cx);
    });
    assert!(events.borrow().is_empty());
    step.update(cx, |step, cx| {
      step.set_name("Renamed".into(), cx);
      step.set_description(Some("desc".into()), cx);
      step.switch_disabled(cx);
    });
    step.read_with(cx, |step, _| {
      assert_eq!(step.name(), "Renamed");
      assert_eq!(step.description().as_deref(), Some("desc"));
      assert!(step.disabled());
    });
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 3]);
  }
}
