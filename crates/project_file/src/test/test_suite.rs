use std::fmt::Debug;

use crate::test::{
    TestInfo,
    test_case::{TestCase, TestCaseInfo},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestSuite {
    #[serde(flatten)]
    pub info: TestInfo,
    pub cases: Vec<TestCase>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TestSuiteInfo {
    #[serde(flatten)]
    pub info: TestInfo,
    pub cases: Vec<TestCaseInfo>,
}
