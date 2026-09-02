use crate::workspace::test::{TestInfo, test_step::TestStep};
use ki_project::FileTestCase;

#[derive(Clone, Debug, Eq)]
pub struct TestCase {
    pub info: TestInfo,
    pub is_anonymous: bool,
    pub steps: Vec<TestStep>,
}

impl TestCase {
    pub fn from_file(file_test_cases: Vec<FileTestCase>) -> Vec<TestCase> {
        file_test_cases
            .into_iter()
            .map(|file_test_suite| Self {
                info: TestInfo::from_file(file_test_suite.info),
                is_anonymous: file_test_suite.is_anonymous,
                steps: TestStep::from_file(file_test_suite.steps),
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

    #[allow(unused)]
    pub fn info_from_path(&self, ix: &usize) -> Option<&TestInfo> {
        self.steps.get(*ix).map(|step| &step.info)
    }

    pub fn info_mut_from_path(&mut self, ix: &usize) -> Option<&mut TestInfo> {
        self.steps.get_mut(*ix).map(|step| &mut step.info)
    }
}

impl PartialEq for TestCase {
    fn eq(&self, other: &Self) -> bool {
        self.info.id == other.info.id || self.info.name == other.info.name
    }
}
