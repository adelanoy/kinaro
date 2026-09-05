use crate::test::{FileTestInfo, test_step::FileTestStep};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileTestCase {
    #[serde(flatten)]
    pub info: FileTestInfo,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_anonymous: bool,
    pub steps: Vec<FileTestStep>,
}
