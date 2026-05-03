use crate::test::{
    test_step::WorkspaceTestStep,
    WorkspaceTestInfo,
};
use project_file::TestCase;

#[derive(Clone, Debug)]
pub struct WorkspaceTestCase {
    pub info: WorkspaceTestInfo,
    pub is_anonymous: bool,
    pub steps: Vec<WorkspaceTestStep>,
}

impl WorkspaceTestCase {
    pub(crate) fn from_file(file_test_cases: &Vec<TestCase>) -> Vec<WorkspaceTestCase> {
        file_test_cases.iter()
            .map(|file_test_suite| {
                Self {
                    info: WorkspaceTestInfo::from_file(&file_test_suite.info),
                    is_anonymous: file_test_suite.is_anonymous,
                    steps: WorkspaceTestStep::from_file(&file_test_suite.steps),
                }
            }).collect()
    }

    pub(crate) fn get_file(&self) -> TestCase {
        TestCase {
            info: self.info.get_file(),
            is_anonymous: self.is_anonymous,
            steps: self.steps.iter().map(|case| case.get_file()).collect(),
        }
    }
}

impl PartialEq for WorkspaceTestCase {
    fn eq(&self, other: &Self) -> bool {
        self.info.id == other.info.id || self.info.name == other.info.name
    }
}
