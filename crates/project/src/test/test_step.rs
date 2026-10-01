use crate::test::FileTestMetadata;
use crate::test::step_delay::FileDelayStep;
use crate::test::step_rest::FileRestStep;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FileStepData {
  RestStep(FileRestStep),
  Delay(FileDelayStep),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestStep {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub data: FileStepData,
}
