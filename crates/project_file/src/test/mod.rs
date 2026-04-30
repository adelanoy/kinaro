use crate::test::test_suite::TestSuite;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod test_case;
pub mod test_step;
pub mod test_suite;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub active: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestsContainer {
    pub suites: Vec<TestSuite>,
}
