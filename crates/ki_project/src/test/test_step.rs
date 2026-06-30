use crate::test::FileTestInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTestStep {
    #[serde(flatten)]
    pub info: FileTestInfo,
    pub data: String,
}