use crate::workspace::test::TestInfo;
use ki_project::FileTestStep;

#[derive(Clone, Debug)]
pub struct TestStep {
    pub info: TestInfo,
    pub data: String,
}

impl TestStep {
    pub fn from_file(file_test_step: &Vec<FileTestStep>) -> Vec<TestStep> {
        file_test_step
            .iter()
            .map(|file_test_suite| Self {
                info: TestInfo::from_file(&file_test_suite.info),
                data: file_test_suite.data.clone(),
            })
            .collect()
    }
    pub fn get_file(&self) -> FileTestStep {
        FileTestStep {
            info: self.info.get_file(),
            data: self.data.clone(),
        }
    }
}

impl PartialEq for TestStep {
    fn eq(&self, other: &Self) -> bool {
        self.info.id == other.info.id || self.info.name == other.info.name
    }
}
