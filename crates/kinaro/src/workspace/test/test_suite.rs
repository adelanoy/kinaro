use std::fmt::Debug;

use crate::workspace::test::{test_case::WorkspaceTestCase, WorkspaceTestInfo};
use project_file::TestSuite;

#[derive(Clone, Debug)]
pub struct WorkspaceTestSuite {
    pub info: WorkspaceTestInfo,
    pub cases: Vec<WorkspaceTestCase>,
}

impl WorkspaceTestSuite {
    pub(crate) fn from_file(file_test_suites: &Vec<TestSuite>) -> Vec<WorkspaceTestSuite> {
        file_test_suites
            .iter()
            .map(|file_test_suite| Self {
                info: WorkspaceTestInfo::from_file(&file_test_suite.info),
                cases: WorkspaceTestCase::from_file(&file_test_suite.cases),
            })
            .collect()
    }

    pub(crate) fn get_file(&self) -> TestSuite {
        TestSuite {
            info: self.info.get_file(),
            cases: self.cases.iter().map(|case| case.get_file()).collect(),
        }
    }
}

impl PartialEq for WorkspaceTestSuite {
    fn eq(&self, other: &Self) -> bool {
        self.info.name == other.info.name && self.cases == other.cases
    }
}
