use crate::test::WorkspaceTestInfo;
use project_file::TestStep;

#[derive(Clone, Debug)]
pub struct WorkspaceTestStep {
    pub info: WorkspaceTestInfo,
    pub data: String,
}

impl WorkspaceTestStep {
    pub(crate) fn from_file(file_test_step: &Vec<TestStep>) -> Vec<WorkspaceTestStep> {
        file_test_step.iter()
            .map(|file_test_suite| {
                Self {
                    info: WorkspaceTestInfo::from_file(&file_test_suite.info),
                    data: file_test_suite.data.clone(),
                }
            }).collect()
    }
    pub(crate) fn get_file(&self) -> TestStep {
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
