mod endpoint;
mod error;
mod test;
mod variable;

pub use {
    endpoint::{FileEndpoint, FileRestParameter, HttpMethod},
    error::ProjectFileError,
    ki_project::ProjectFile,
    test::{
        FileTestInfo, FileTestsContainer, test_case::FileTestCase, test_step::FileTestStep,
        test_suite::FileTestSuite,
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
  use std::path::{Path, PathBuf};

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
        pub fn create(path: &Path, name: &str) -> Result<ProjectFile> {
            check_project_path(path, false)
                .and_then(|()| check_project_name(name))
                .map(|()| ProjectFile {
                    name: name.into(),
                    version: 1,
                    created: Local::now(),
                    modified: Local::now(),
                    variables: FileProjectVariables::default(),
                    endpoints: vec![],
                    tests: FileTestsContainer::default(),
                })
        }

        pub fn load(path: &Path) -> Result<ProjectFile> {
            check_project_path(path, true)
                .and_then(|()| fs::read(path).map_err(ProjectFileError::Io))
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
                .map_err(ProjectFileError::from)
                .and_then(|file| {
                    serde_yaml::to_writer(file, &self)
                        .map_err(|err| ProjectFileError::WriteYaml(err.to_string()))
                })
        }
    }

    fn check_project_path(path: &Path, should_exist: bool) -> Result<()> {
        if path.exists() {
            if !path.is_file() {
                return Err(ProjectFileError::BadLocation(path.to_owned()));
            }
        } else if should_exist {
            return Err(ProjectFileError::BadLocation(path.to_owned()));
        }
        match path.extension() {
            Some(ext) => {
                if ext == "kpr" {
                    Ok(())
                } else {
                    Err(ProjectFileError::BadLocation(path.to_owned()))
                }
            }
            None => Err(ProjectFileError::BadLocation(path.to_owned())),
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
