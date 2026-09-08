use std::fmt::Debug;

use crate::test::{FileTestInfo, test_case::FileTestCase};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestSuite {
  #[serde(flatten)]
  pub info: FileTestInfo,
  pub cases: Vec<FileTestCase>,
}
