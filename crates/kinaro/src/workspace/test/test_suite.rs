use std::fmt::Debug;

use crate::workspace::test::{test_case::TestCase, TestInfo};
use ki_project::FileTestSuite;

#[derive(Clone, Debug, Eq)]
pub struct TestSuite {
    pub info: TestInfo,
    pub cases: Vec<TestCase>,
}

impl TestSuite {
    pub fn from_file(file_test_suites: Vec<FileTestSuite>) -> Vec<TestSuite> {
        file_test_suites
            .into_iter()
            .map(|file_test_suite| Self {
                info: TestInfo::from_file(file_test_suite.info),
                cases: TestCase::from_file(file_test_suite.cases),
            })
            .collect()
    }

    pub fn get_file(&self) -> FileTestSuite {
        FileTestSuite {
            info: self.info.get_file(),
            cases: self.cases.iter().map(|case| case.get_file()).collect(),
        }
    }
}

impl PartialEq for TestSuite {
    fn eq(&self, other: &Self) -> bool {
        self.info.name == other.info.name && self.cases == other.cases
    }
}
