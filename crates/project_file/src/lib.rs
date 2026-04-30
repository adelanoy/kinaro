mod date_time;
mod endpoint;
mod error;
mod test;
mod variable;

pub use {
    endpoint::{Endpoint, HttpMethod, RestParameter},
    error::ProjectFileError,
    project_file::ProjectFile,
    test::{
        TestInfo, TestsContainer,
        test_suite::{TestSuite, TestSuiteInfo},
        test_case::{TestCase, TestCaseInfo},
        test_step::{TestStep, TestStepInfo},
    },
    variable::{Profile, VariableKind, Variables, Variable},
};

mod project_file {
    use crate::date_time::java_date_format;
    use crate::endpoint::Endpoint;
    use crate::error::{ProjectFileError, Result};
    use crate::test::TestsContainer;
    use crate::variable::Variables;
    use chrono::{DateTime, Local};
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ProjectFile {
        pub name: String,
        pub id: Uuid,
        pub version: u16,
        #[serde(with = "java_date_format")]
        pub created: DateTime<Local>,
        #[serde(with = "java_date_format")]
        pub modified: DateTime<Local>,
        pub variables: Variables,
        pub endpoints: Vec<Endpoint>,
        pub tests: TestsContainer,
    }

    impl ProjectFile {
        pub fn create(path: &PathBuf, name: &str) -> Result<ProjectFile> {
            check_project_path(path, false)
                .and_then(|_| check_project_name(name))
                .map(|_| ProjectFile {
                    id: Uuid::new_v4(),
                    name: name.into(),
                    version: 1,
                    created: Local::now(),
                    modified: Local::now(),
                    variables: Variables::default(),
                    endpoints: vec![],
                    tests: TestsContainer::default(),
                })
        }

        pub fn load(path: &PathBuf) -> Result<ProjectFile> {
            check_project_path(path, true)
                .and_then(|_| fs::read(&path).map_err(|e| ProjectFileError::Io(e)))
                .and_then(|file| {
                    serde_yaml::from_slice::<ProjectFile>(&file)
                        .map_err(|err| ProjectFileError::ReadYaml(err.to_string()))
                })
        }

        pub fn save(&self, path: &PathBuf) -> Result<()> {
            fs::File::create(path)
                .map_err(|err| ProjectFileError::from(err))
                .and_then(|file| {
                    serde_yaml::to_writer(file, &self)
                        .map_err(|err| ProjectFileError::WriteYaml(err.to_string()))
                })
        }
    }

    fn check_project_path(path: &PathBuf, should_exist: bool) -> Result<()> {
        if path.exists() {
            if !path.is_file() {
                return Err(ProjectFileError::BadLocation(path.clone()));
            }
        } else if should_exist {
            return Err(ProjectFileError::BadLocation(path.clone()));
        }
        match path.extension() {
            Some(ext) => {
                if ext == "kpr" {
                    Ok(())
                } else {
                    Err(ProjectFileError::BadLocation(path.clone()))
                }
            }
            None => Err(ProjectFileError::BadLocation(path.clone())),
        }
    }

    fn check_project_name(name: &str) -> Result<()> {
        if name.is_empty() {
            Err(ProjectFileError::InvalidName(name.into()))
        } else {
            Ok(())
        }
    }
}
