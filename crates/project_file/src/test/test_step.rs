use crate::test::TestInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestStep {
    #[serde(flatten)]
    pub info: TestInfo,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TestStepInfo {
    #[serde(flatten)]
    pub info: TestInfo,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StepData {
    Rest(RestStep),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RestStep {}
