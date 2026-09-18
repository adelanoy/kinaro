use crate::test::FileTestMetadata;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestStep {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub data: String,
}
