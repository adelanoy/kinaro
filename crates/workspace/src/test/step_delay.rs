use ki_project::FileDelayStep;

/// A step waiting for the given number of milliseconds.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct DelayStep(pub u32);

impl From<FileDelayStep> for DelayStep {
  fn from(value: FileDelayStep) -> Self {
    Self(value.0)
  }
}

impl From<&DelayStep> for FileDelayStep {
  fn from(value: &DelayStep) -> Self {
    Self(value.0)
  }
}
