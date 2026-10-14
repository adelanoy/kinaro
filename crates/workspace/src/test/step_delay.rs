use ki_project::FileDelayStep;
use ki_utils::time::TimeUnit;

/// A step waiting for `duration` expressed in `unit` (e.g. 3 seconds).
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct DelayData {
  /// How long to wait, in `unit`
  pub duration: u32,
  /// The time unit `duration` is expressed in
  pub unit: TimeUnit,
}

impl From<FileDelayStep> for DelayData {
  fn from(value: FileDelayStep) -> Self {
    Self {
      duration: value.duration,
      unit: value.unit,
    }
  }
}

impl From<&DelayData> for FileDelayStep {
  fn from(value: &DelayData) -> Self {
    Self {
      duration: value.duration,
      unit: value.unit,
    }
  }
}
