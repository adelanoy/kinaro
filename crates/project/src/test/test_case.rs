use crate::test::{FileTestMetadata, test_step::FileTestStep};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestCase {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub steps: Vec<FileTestStep>,
}
