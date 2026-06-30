use crate::workspace::test::{test_step::TestStep, TestInfo};
use ki_project::FileTestCase;

#[derive(Clone, Debug)]
pub struct TestCase {
    pub info: TestInfo,
    pub is_anonymous: bool,
    pub steps: Vec<TestStep>,
}

impl TestCase {
    pub fn from_file(file_test_cases: &Vec<FileTestCase>) -> Vec<TestCase> {
        file_test_cases
            .iter()
            .map(|file_test_suite| Self {
                info: TestInfo::from_file(&file_test_suite.info),
                is_anonymous: file_test_suite.is_anonymous,
                steps: TestStep::from_file(&file_test_suite.steps),
            })
            .collect()
    }

    pub fn get_file(&self) -> FileTestCase {
        FileTestCase {
            info: self.info.get_file(),
            is_anonymous: self.is_anonymous,
            steps: self.steps.iter().map(|case| case.get_file()).collect(),
        }
    }
}

impl PartialEq for TestCase {
    fn eq(&self, other: &Self) -> bool {
        self.info.id == other.info.id || self.info.name == other.info.name
    }
}
