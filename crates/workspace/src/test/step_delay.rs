use crate::test::{TestsContainer, TestsContainerEvent};
use gpui_kit::{Context, WeakEntity};
use ki_project::FileDelayData;
use ki_utils::time::TimeUnit;

/// A step waiting for `duration` expressed in `unit` (e.g. 3 seconds).
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct DelayData {
  /// The container holding this step, notified of the changes made to the step
  pub(crate) container: WeakEntity<TestsContainer>,
  /// How long to wait, in `unit`
  pub(crate) duration: u32,
  /// The time unit `duration` is expressed in
  pub(crate) unit: TimeUnit,
}

impl DelayData {
  #[inline]
  pub fn duration(&self) -> u32 {
    self.duration
  }

  pub fn get_file(&self) -> FileDelayData {
    FileDelayData {
      duration: self.duration,
      unit: self.unit,
    }
  }

  pub fn from_file(value: FileDelayData, container: WeakEntity<TestsContainer>) -> Self {
    Self {
      container,
      duration: value.duration,
      unit: value.unit,
    }
  }

  #[inline]
  pub fn unit(&self) -> TimeUnit {
    self.unit
  }

  pub fn update(&mut self, duration: u32, unit: TimeUnit, cx: &mut Context<Self>) {
    let mut update = false;
    if duration != self.duration {
      self.duration = duration;
      update = true;
    }
    if unit != self.unit {
      self.unit = unit;
      update = true;
    }
    if update {
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    }
  }
}
