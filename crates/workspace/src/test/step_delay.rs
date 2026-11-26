use crate::TestStep;
use crate::test::test_step::StepData;
use crate::test::{TestsContainer, TestsContainerEvent};
use gpui_kit::{Context, EventEmitter, WeakEntity};
use ki_project::FileDelayData;
use ki_utils::ExprValue;
use ki_utils::time::TimeUnit;
use ki_utils::undo::Edit;

pub enum DelayDataEvent {
  DataChanged,
}

/// A step waiting for `duration` expressed in `unit` (e.g. 3 seconds).
///
/// Not `Clone`: a copy must be attached to its own step, see [`DelayData::duplicate`].
#[derive(Eq, PartialEq, Debug)]
pub struct DelayData {
  /// The container holding this step, notified of the changes made to the step
  pub(crate) container: WeakEntity<TestsContainer>,
  /// The parent step, to allow pushing undo/redo edits
  pub(crate) parent: WeakEntity<TestStep>,
  /// How long to wait, in `unit`
  pub(crate) duration: ExprValue<u32>,
  /// The time unit `duration` is expressed in
  pub(crate) unit: TimeUnit,
}

impl DelayData {
  /// Returns a copy of this delay, owned by the step `parent`.
  pub(crate) fn duplicate(&self, parent: WeakEntity<TestStep>) -> Self {
    Self {
      container: self.container.clone(),
      parent,
      duration: self.duration.clone(),
      unit: self.unit,
    }
  }

  #[inline]
  pub fn duration(&self) -> ExprValue<u32> {
    self.duration.clone()
  }

  pub fn get_file(&self) -> FileDelayData {
    FileDelayData {
      duration: (&self.duration).into(),
      unit: self.unit,
    }
  }

  pub fn from_file(value: FileDelayData, parent: WeakEntity<TestStep>, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      parent,
      duration: value.duration.into(),
      unit: value.unit,
    }
  }

  #[inline]
  pub fn unit(&self) -> TimeUnit {
    self.unit
  }

  pub fn update(&mut self, duration: ExprValue<u32>, unit: TimeUnit, cx: &mut Context<Self>) {
    let before = (self.duration.clone(), self.unit);
    self.duration = duration;
    self.unit = unit;
    let after = (self.duration.clone(), self.unit);
    if before != after {
      _ = self
        .parent
        .update(cx, |parent, _| parent.push_edit(Box::new(EditDelay { before, after })));
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
      cx.notify();
    }
  }
}

impl EventEmitter<DelayDataEvent> for DelayData {}

#[derive(PartialEq, Clone, Debug)]
struct EditDelay {
  before: (ExprValue<u32>, TimeUnit),
  after: (ExprValue<u32>, TimeUnit),
}

impl EditDelay {
  fn apply(&self, target: &mut TestStep, data: &(ExprValue<u32>, TimeUnit), cx: &mut Context<TestStep>) {
    if let StepData::Delay(delay) = &target.data {
      delay.update(cx, |delay, cx| {
        delay.duration = data.0.clone();
        delay.unit = data.1;
        _ = delay
          .container
          .update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
        cx.emit(DelayDataEvent::DataChanged);
        cx.notify();
      });
    }
  }
}

impl Edit<TestStep> for EditDelay {
  fn undo(&self, target: &mut TestStep, cx: &mut Context<TestStep>) {
    self.apply(target, &self.before, cx);
  }

  fn redo(&self, target: &mut TestStep, cx: &mut Context<TestStep>) {
    self.apply(target, &self.after, cx);
  }
}
