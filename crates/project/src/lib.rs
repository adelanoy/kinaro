mod error;
mod test;
mod variable;

pub use {
  error::ProjectFileError,
  ki_project::ProjectFile,
  test::{
    FileTestMetadata, FileTestsContainer,
    step_delay::FileDelayData,
    step_rest::*,
    test_case::FileTestCase,
    test_step::{FileStepData, FileTestStep},
    test_suite::{FileSuiteChild, FileTestSuite},
  },
  variable::{FileProfile, FileProjectVariables, FileVariable, FileVariableKind},
};

mod ki_project {
  use crate::error::{ProjectFileError, ProjectValidationError, Result};
  use crate::test::FileTestsContainer;
  use crate::variable::FileProjectVariables;
  use jiff::ZonedDateTime;
  use ki_utils::serial_date_format;
  use serde::{Deserialize, Serialize};
  use std::fs;
  use std::path::{Path, PathBuf};

  #[derive(Serialize, Deserialize, Clone, Debug)]
  #[serde(rename_all = "camelCase")]
  pub struct ProjectFile {
    pub name: String,
    pub version: u16,
    #[serde(with = "serial_date_format")]
    pub created: ZonedDateTime,
    #[serde(with = "serial_date_format")]
    pub modified: ZonedDateTime,
    pub variables: FileProjectVariables,
    pub tests: FileTestsContainer,
  }

  impl ProjectFile {
    /// Checks the consistency of the project, returning it unchanged when valid
    ///
    /// # Errors
    /// Returns a [`ProjectFileError::ValidationFailed`] listing every problem found
    pub fn check(self) -> Result<Self> {
      let mut errors: Vec<ProjectValidationError> = vec![];
      errors.extend(self.variables.check());

      if errors.is_empty() {
        Ok(self)
      } else {
        Err(ProjectFileError::ValidationFailed(errors))
      }
    }

    pub fn create(path: &Path, name: &str) -> Result<Self> {
      check_project_path(path, false)
        .and_then(|()| check_project_name(name))
        .map(|()| ProjectFile {
          name: name.into(),
          version: 1,
          created: ZonedDateTime::now(),
          modified: ZonedDateTime::now(),
          variables: FileProjectVariables::default(),
          tests: FileTestsContainer::default(),
        })
    }

    pub fn load(path: &Path) -> Result<Self> {
      check_project_path(path, true)
        .and_then(|()| fs::read(path).map_err(ProjectFileError::Io))
        .and_then(|file| serde_json::from_slice::<ProjectFile>(&file).map_err(|err| ProjectFileError::Read(err.to_string())))
        .and_then(Self::check)
    }

    pub fn save(&self, path: &PathBuf) -> Result<ZonedDateTime> {
      fs::File::create(path).map_err(ProjectFileError::from).and_then(|file| {
        let date_time = self.modified.clone();
        serde_json::to_writer_pretty(file, &self)
          .map_err(|err| ProjectFileError::Write(err.to_string()))
          .map(|()| date_time)
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
      Err(ProjectFileError::InvalidProjectName(name.into()))
    } else {
      Ok(())
    }
  }
}
