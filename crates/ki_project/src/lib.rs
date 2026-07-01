mod endpoint;
mod error;
mod test;
mod variable;

pub use {
    endpoint::{FileEndpoint, FileRestParameter, HttpMethod},
    error::ProjectFileError,
    ki_project::ProjectFile,
    test::{
        test_case::FileTestCase, test_step::FileTestStep, test_suite::FileTestSuite, FileTestInfo,
        FileTestsContainer,
    },
    variable::{FileProfile, FileProjectVariables, FileVariable, VariableKind},
};

mod ki_project {
    use crate::endpoint::FileEndpoint;
    use crate::error::{ProjectFileError, Result};
    use crate::test::FileTestsContainer;
    use crate::variable::FileProjectVariables;
    use chrono::{DateTime, Local};
    use ki_utils::java_date_format;
    use log::debug;
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::PathBuf;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ProjectFile {
        pub name: String,
        pub version: u16,
        #[serde(with = "java_date_format")]
        pub created: DateTime<Local>,
        #[serde(with = "java_date_format")]
        pub modified: DateTime<Local>,
        pub variables: FileProjectVariables,
        pub endpoints: Vec<FileEndpoint>,
        pub tests: FileTestsContainer,
    }

    impl ProjectFile {
        pub fn create(path: &PathBuf, name: &str) -> Result<ProjectFile> {
            check_project_path(path, false)
                .and_then(|_| check_project_name(name))
                .map(|_| ProjectFile {
                    name: name.into(),
                    version: 1,
                    created: Local::now(),
                    modified: Local::now(),
                    variables: FileProjectVariables::default(),
                    endpoints: vec![],
                    tests: FileTestsContainer::default(),
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
            debug!(
                "Saving project file {} to {}",
                self.name,
                path.to_string_lossy()
            );
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
