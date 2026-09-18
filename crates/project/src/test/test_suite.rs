use std::fmt::Debug;

use crate::test::{FileTestMetadata, test_case::FileTestCase};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestSuite {
  #[serde(flatten)]
  pub info: FileTestMetadata,
  pub cases: Vec<FileTestCase>,
}
