use crate::test::{
    test_step::{TestStep, TestStepInfo},
    TestInfo,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    #[serde(flatten)]
    pub info: TestInfo,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_anonymous: bool,
    pub steps: Vec<TestStep>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseInfo {
    #[serde(flatten)]
    pub info: TestInfo,
    pub is_anonymous: bool,
    pub steps: Vec<TestStepInfo>,
}
