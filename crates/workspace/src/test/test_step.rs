use crate::TestCase;
use crate::test::step_delay::DelayStep;
use crate::test::step_rest::RestStep;
use crate::test::test_case::TestCaseType;
use crate::test::{StepDataProvider, TestMetadata, TestMetadataOwner, TestPath, TestsContainer, TestsContainerEvent};
use gpui_kit::component::Icon;
use gpui_kit::{App, Context, Entity, SharedString, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileStepData, FileTestCase, FileTestCaseType, FileTestStep};
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
  pub fn icon(&self) -> Icon {
    match self {
      StepData::RestStep(_) => IconAsset::StepRest,
      StepData::Delay(_) => IconAsset::StepDelay,
    }.into()
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
/// own steps, or a case step promoted directly to case level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestStep {
  /// The container holding this step, notified of the changes made to the step
  container: WeakEntity<TestsContainer>,
  /// Metadata for the step itself (id, name, description, disabled flag).
  ///
  /// `meta.path` is always a [`TestPath::Step`].
  pub meta: TestMetadata,
  /// What the step does when run
  pub data: StepData,
}

impl TestStep {
  /// Returns a copy of this step named `name`, placed in the case `parent_path` goes through, with
  /// a fresh id.
  ///
  /// # Panics
  /// If `parent_path` is a suite path.
  pub(crate) fn duplicate(&self, name: SharedString, parent_path: &TestPath) -> Self {
    let path = TestPath::Step(parent_path.suite_id(), parent_path.case_id().unwrap(), Uuid::new_v4());
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

  /// Demotes the case step `case` to a step of the case `parent_path` goes through, keeping the
  /// case's id. Returns `None` if `case` is a multi-step case.
  ///
  /// # Panics
  /// If `parent_path` is a suite path.
  pub(crate) fn from_case(case: Entity<TestCase>, parent_path: &TestPath, cx: &App) -> Option<Self> {
    case.read_with(cx, |case, _| {
      if let TestCaseType::CaseStep { data } = &case.case_type {
        Some(Self {
          container: case.container.clone(),
          meta: TestMetadata {
            path: TestPath::Step(parent_path.suite_id(), parent_path.case_id().unwrap(), case.id()),
            name: case.name(),
            description: case.description(),
            disabled: case.disabled(),
          },
          data: data.clone(),
        })
      } else {
        None
      }
    })
  }

  /// Builds a step of the case at `parent_path`, with a fresh id, from the on-disk representation
  /// of a case step. Returns `None` if `parent_path` isn't a case path or `file_case` is a
  /// multi-step case.
  pub(crate) fn from_case_file(
    file_case: FileTestCase,
    parent_path: TestPath,
    container: WeakEntity<TestsContainer>,
  ) -> Option<Self> {
    if !parent_path.is_case() {
      return None;
    }
    match file_case.case_type {
      FileTestCaseType::CaseMulti { .. } => None,
      FileTestCaseType::CaseStep { data } => Some(Self {
        container,
        meta: TestMetadata::new_step_from_file(file_case.info, parent_path.suite_id(), parent_path.case_id().unwrap()),
        data: data.into(),
      }),
    }
  }

  /// Builds a step of the case `parent_path` goes through from its on-disk representation. With
  /// `new_id`, the step gets a freshly generated id (e.g. when pasted); otherwise its id is read
  /// from the file.
  ///
  /// # Panics
  /// If `parent_path` is a suite path.
  pub(crate) fn from_file(
    file_step: FileTestStep,
    parent_path: TestPath,
    container: WeakEntity<TestsContainer>,
    new_id: bool,
  ) -> TestStep {
    Self {
      container,
      meta: if new_id {
        TestMetadata::new_step_from_file(file_step.info, parent_path.suite_id(), parent_path.case_id().unwrap())
      } else {
        TestMetadata::step_from_file(file_step.info, parent_path.suite_id(), parent_path.case_id().unwrap())
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

  /// Creates a new delay step of 1000 ms with a freshly generated id.
  pub fn new_delay(suite_id: Uuid, case_id: Uuid, name: SharedString, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      meta: TestMetadata {
        path: TestPath::Step(suite_id, case_id, Uuid::new_v4()),
        name,
        description: None,
        disabled: false,
      },
      data: StepData::Delay(DelayStep(1000)),
    }
  }

  /// Moves this step under the case `parent` goes through. With `keep_id` the id is kept,
  /// otherwise a fresh one is generated. Returns the path change, as `(old path, new path)`.
  ///
  /// # Panics
  /// If `parent` is a suite path.
  pub(crate) fn reparent(&mut self, parent: &TestPath, keep_id: bool) -> (TestPath, TestPath) {
    let id = if keep_id { self.meta.id() } else { Uuid::new_v4() };
    let new_path = TestPath::Step(parent.suite_id(), parent.case_id().unwrap(), id);
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

  fn icon(&self) -> Icon {
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

impl StepDataProvider for TestStep {
  fn data(&self) -> Option<&StepData> {
    Some(&self.data)
  }

  fn data_mut(&mut self) -> Option<&mut StepData> {
    Some(&mut self.data)
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
    let step = TestStep::new_delay(suite_id, case_id, "Delay".into(), tests.downgrade());
    assert!(matches!(step.path(), TestPath::Step(suite, case, _) if suite == suite_id && case == case_id));
    assert_eq!(step.name(), "Delay");
    assert_eq!(step.data, StepData::Delay(DelayStep(1000)));
    assert_ne!(
      TestStep::new_delay(suite_id, case_id, "Delay".into(), tests.downgrade()).id(),
      step.id()
    );
  }

  #[gpui_kit::test]
  fn from_file_keeps_or_renews_the_id(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let parent = TestPath::Case(Uuid::new_v4(), Uuid::new_v4());
    let step_id = Uuid::new_v4();
    let file = file_delay_step(step_id, "Step", 10);
    let kept = TestStep::from_file(file.clone(), parent, tests.downgrade(), false);
    assert_eq!(kept.path(), TestPath::Step(parent.suite_id(), parent.id(), step_id));
    assert_eq!(kept.data, StepData::Delay(DelayStep(10)));
    let renewed = TestStep::from_file(file, parent, tests.downgrade(), true);
    assert!(parent.is_parent(&renewed.path()));
    assert_ne!(renewed.id(), step_id);
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
    let step = TestStep::from_file(file.clone(), parent, tests.downgrade(), false);
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
  fn from_case_demotes_a_case_step_keeping_its_id(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_step = case(&tests, p.case_step_a, cx);
    let step = cx.read(|cx| TestStep::from_case(case_step, &p.case_b, cx)).unwrap();
    assert_eq!(step.path(), TestPath::Step(p.suite_b.id(), p.case_b.id(), p.case_step_a.id()));
    assert_eq!(step.name(), "Case step A");
    assert_eq!(step.data, StepData::Delay(DelayStep(300)));

    let multi_case = case(&tests, p.case_a, cx);
    assert!(cx.read(|cx| TestStep::from_case(multi_case, &p.case_b, cx)).is_none());
  }

  #[gpui_kit::test]
  fn from_case_file_accepts_case_steps_in_cases_only(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let file = p.file();
    let case_step = file.suites[0].cases[1].clone();
    let step = TestStep::from_case_file(case_step.clone(), p.case_b, tests.downgrade()).unwrap();
    assert!(p.case_b.is_parent(&step.path()));
    assert_ne!(step.id(), p.case_step_a.id());
    assert_eq!(step.data, StepData::Delay(DelayStep(300)));

    assert!(TestStep::from_case_file(case_step, p.suite_b, tests.downgrade()).is_none());
    assert!(TestStep::from_case_file(file.suites[0].cases[0].clone(), p.case_b, tests.downgrade()).is_none());
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
