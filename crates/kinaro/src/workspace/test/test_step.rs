use crate::workspace::test::WorkspaceTestInfo;
use ki_project::TestStep;

#[derive(Clone, Debug)]
pub struct WorkspaceTestStep {
    pub info: WorkspaceTestInfo,
    pub data: String,
}

impl WorkspaceTestStep {
    pub fn from_file(file_test_step: &Vec<TestStep>) -> Vec<WorkspaceTestStep> {
        file_test_step
            .iter()
            .map(|file_test_suite| Self {
                info: WorkspaceTestInfo::from_file(&file_test_suite.info),
                data: file_test_suite.data.clone(),
            })
            .collect()
    }
    pub fn get_file(&self) -> TestStep {
        TestStep {
            info: self.info.get_file(),
            data: self.data.clone(),
        }
    }
}

impl PartialEq for WorkspaceTestStep {
    fn eq(&self, other: &Self) -> bool {
        self.info.id == other.info.id || self.info.name == other.info.name
    }
}
