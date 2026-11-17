use crate::test::FileTestMetadata;
use crate::test::step_delay::FileDelayData;
use crate::test::step_rest::FileRestData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FileStepData {
  Rest(FileRestData),
  Delay(FileDelayData),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestStep {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub data: FileStepData,
}
