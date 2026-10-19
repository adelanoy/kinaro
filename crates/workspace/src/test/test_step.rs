use crate::test::step_delay::DelayData;
use crate::test::step_rest::RestData;
use crate::test::{TestMetadata, TestMetadataOwner, TestPath, TestsContainer, TestsContainerEvent};
use gpui_kit::component::Icon;
use gpui_kit::{App, AppContext, Context, Entity, Rgba, SharedString, Styled, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileStepData, FileTestStep};
use ki_utils::time::TimeUnit;
use uuid::Uuid;

/// What a step does when run, one variant per kind of step.
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum StepData {
  /// Sends a REST request
  Rest(Entity<RestData>),
  /// Waits for a given time
  Delay(Entity<DelayData>),
}

impl StepData {
  fn duplicate(&self, cx: &mut App) -> StepData {
    match self {
      StepData::Rest(data) => StepData::Rest(cx.new(|cx| data.read(cx).clone())),
      StepData::Delay(data) => StepData::Delay(cx.new(|cx| data.read(cx).clone())),
    }
  }

  /// Returns the icon of this kind of step.
  pub fn icon(&self, cx: &App) -> Icon {
    match self {
      StepData::Rest(data) => data.read(cx).icon(),
      StepData::Delay(_) => Icon::new(IconAsset::StepDelay).text_color(Rgba::try_from("#f59e0b").unwrap()),
    }
  }

  fn from_file(file: FileStepData, container: WeakEntity<TestsContainer>, cx: &mut App) -> Self {
    match file {
      FileStepData::RestStep(rest_step) => StepData::Rest(cx.new(|_| RestData::from_file(rest_step, container))),
      FileStepData::Delay(delay_step) => StepData::Delay(cx.new(|_| DelayData::from_file(delay_step, container))),
    }
  }

  fn get_file(&self, cx: &App) -> FileStepData {
    match self {
      StepData::Rest(rest_step) => FileStepData::RestStep(rest_step.read(cx).get_file()),
      StepData::Delay(delay_step) => FileStepData::Delay(delay_step.read(cx).get_file()),
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

  /// Edits what the step does when run: `f` is given the step data and returns whether it
  /// changed it.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if `f` returned `true`.
  pub fn update_data(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut StepData) -> bool) {
    if f(&mut self.data) {
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    }
  }

  /// Returns a copy of this step named `name`, placed where `parent_path` points, with a fresh id.
  pub(crate) fn duplicate(&self, name: SharedString, parent_path: &TestPath, cx: &mut Context<Self>) -> Self {
    let path = TestPath::new_step(parent_path, Uuid::new_v4());
    let data = self.data.duplicate(cx);
    Self {
      container: self.container.clone(),
      meta: TestMetadata {
        path,
        name,
        description: self.meta.description.clone(),
        disabled: self.meta.disabled,
      },
      data,
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
    cx: &mut Context<Self>,
  ) -> TestStep {
    Self {
      container: container.clone(),
      meta: if new_id {
        TestMetadata::new_step_from_file(file_step.info, parent_path)
      } else {
        TestMetadata::step_from_file(file_step.info, parent_path)
      },
      data: StepData::from_file(file_step.data, container, cx),
    }
  }

  /// Produces the serializable, on-disk representation of this step.
  pub fn get_file(&self, cx: &App) -> FileTestStep {
    FileTestStep {
      info: self.meta.to_file(),
      data: self.data.get_file(cx),
    }
  }

  /// Creates a new delay step of 1000 ms placed where `parent_path` points, with a freshly
  /// generated id.
  pub fn new_delay(parent_path: &TestPath, name: SharedString, container: WeakEntity<TestsContainer>, cx: &mut App) -> Self {
    Self {
      container: container.clone(),
      meta: TestMetadata {
        path: TestPath::new_step(parent_path, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      data: StepData::Delay(cx.new(|_| DelayData {
        container,
        duration: 1000,
        unit: TimeUnit::Millisecond,
      })),
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

  fn icon(&self, cx: &App) -> Icon {
    self.data.icon(cx)
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
  use ki_project::{FileDelayData, FileRestBody, FileRestData, FileRestMethod};

  #[gpui_kit::test]
  fn new_delay_has_a_fresh_id(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (suite_id, case_id) = (Uuid::new_v4(), Uuid::new_v4());
    let step = cx.new(|cx| TestStep::new_delay(&TestPath::Case(suite_id, case_id), "Delay".into(), tests.downgrade(), cx));
    step.read_with(cx, |step, _| {
      assert!(matches!(step.path(), TestPath::Step(suite, case, _) if suite == suite_id && case == case_id));
      assert_eq!(step.name(), "Delay");
    });
    assert_eq!(delay_of(&step, cx), (1000, TimeUnit::Millisecond));
    let suite_step = cx.new(|cx| TestStep::new_delay(&TestPath::Suite(suite_id), "Delay".into(), tests.downgrade(), cx));
    let suite_step_path = suite_step.read_with(cx, |step, _| step.path());
    assert!(matches!(suite_step_path, TestPath::SuiteStep(suite, _) if suite == suite_id));
    assert_ne!(suite_step_path.id(), step.read_with(cx, |step, _| step.id()));
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
      let kept = cx.new(|cx| TestStep::from_file(file.clone(), &parent, tests.downgrade(), false, cx));
      assert_eq!(
        kept.read_with(cx, |step, _| step.path()),
        TestPath::new_step(&parent, step_id)
      );
      assert_eq!(delay_of(&kept, cx), (10, TimeUnit::Millisecond));
      let renewed = cx.new(|cx| TestStep::from_file(file.clone(), &parent, tests.downgrade(), true, cx));
      let renewed = renewed.read_with(cx, |step, _| step.path());
      assert!(parent.is_parent(&renewed));
      assert_ne!(renewed.id(), step_id);
    }
  }

  #[gpui_kit::test]
  fn get_file_round_trips_a_rest_step(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let file = FileTestStep {
      info: file_meta(Uuid::new_v4(), "Rest"),
      data: FileStepData::RestStep(FileRestData {
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
    let step = cx.new(|cx| TestStep::from_file(file.clone(), &parent, tests.downgrade(), false, cx));
    cx.read(|cx| {
      let step = step.read(cx);
      let StepData::Rest(rest) = step.data() else {
        panic!("should be a rest step");
      };
      let rest = rest.read(cx);
      assert_eq!(rest.method, RestMethod::Post);
      assert!(matches!(rest.body, RestBody::Raw { .. }));
      assert_eq!(
        serde_json::to_value(step.get_file(cx)).unwrap(),
        serde_json::to_value(file).unwrap()
      );
    });
  }

  #[gpui_kit::test]
  fn get_file_round_trips_a_delay_step(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let file = FileTestStep {
      info: file_meta(Uuid::new_v4(), "Delay"),
      data: FileStepData::Delay(FileDelayData {
        duration: 3,
        unit: TimeUnit::Second,
      }),
    };
    let step = cx.new(|cx| TestStep::from_file(file.clone(), &TestPath::Suite(Uuid::new_v4()), tests.downgrade(), false, cx));
    assert_eq!(delay_of(&step, cx), (3, TimeUnit::Second));
    assert_eq!(
      serde_json::to_value(step.read_with(cx, |step, cx| step.get_file(cx))).unwrap(),
      serde_json::to_value(file).unwrap()
    );
  }

  #[gpui_kit::test]
  fn update_data_notifies_changes_only(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let step = step(&tests, p.step_a1, cx);
    let minute = cx.new(|_| DelayData {
      container: tests.downgrade(),
      duration: 2,
      unit: TimeUnit::Minute,
    });
    let original = step.read_with(cx, |step, _| step.data().clone());
    step.update(cx, |step, cx| step.update_data(cx, |_| false));
    assert_eq!(step.read_with(cx, |step, _| step.data().clone()), original);
    assert!(events.borrow().is_empty());
    step.update(cx, |step, cx| {
      step.update_data(cx, |data| {
        *data = StepData::Delay(minute.clone());
        true
      })
    });
    assert_eq!(delay_of(&step, cx), (2, TimeUnit::Minute));
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn update_delay_notifies_changes_only(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let step = step(&tests, p.step_a1, cx);
    let StepData::Delay(delay) = step.read_with(cx, |step, _| step.data().clone()) else {
      panic!("should be a delay step");
    };
    delay.update(cx, |delay, cx| delay.update(100, TimeUnit::Millisecond, cx));
    assert!(events.borrow().is_empty());
    delay.update(cx, |delay, cx| delay.update(2, TimeUnit::Minute, cx));
    assert_eq!(delay_of(&step, cx), (2, TimeUnit::Minute));
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn duplicate_gives_a_fresh_id_in_the_parent(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let copy = step(&tests, p.step_a1, cx).update(cx, |step, cx| step.duplicate("Copy".into(), &p.case_b, cx));
    assert_eq!(copy.name(), "Copy");
    assert!(p.case_b.is_parent(&copy.path()));
    assert_ne!(copy.id(), p.step_a1.id());
    assert_eq!(delay_of(&cx.new(|_| copy), cx), (100, TimeUnit::Millisecond));
    let copy = step(&tests, p.step_a1, cx).update(cx, |step, cx| step.duplicate("Copy".into(), &p.suite_b, cx));
    assert!(matches!(copy.path(), TestPath::SuiteStep(suite, _) if suite == p.suite_b.id()));
  }

  #[gpui_kit::test]
  fn duplicate_does_not_share_the_data(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let original = step(&tests, p.step_a1, cx);
    let copy = original.update(cx, |step, cx| step.duplicate("Copy".into(), &p.case_a, cx));
    let StepData::Delay(delay) = copy.data().clone() else {
      panic!("should be a delay step");
    };
    delay.update(cx, |delay, cx| delay.update(2, TimeUnit::Minute, cx));
    assert_eq!(delay_of(&cx.new(|_| copy), cx), (2, TimeUnit::Minute));
    assert_eq!(delay_of(&original, cx), (100, TimeUnit::Millisecond));
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
