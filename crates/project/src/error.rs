use std::io;
use std::path::PathBuf;
use ki_utils::id::KId;

/// Standard result for Project plugin
pub type Result<T> = std::result::Result<T, ProjectFileError>;

/// GLOBAL ERROR TYPES
const WRITE_ERROR: &str = "Write";
const READ_ERROR: &str = "Read";
const INVALID_PROJECT_NAME_ERROR: &str = "InvalidProjectName";
const INVALID_VARIABLE_NAME_ERROR: &str = "InvalidVariableName";
const DUPLICATE_VARIABLE_NAME_ERROR: &str = "DuplicateVariableName";
const INVALID_OVERRIDE_ERROR: &str = "InvalidOverride";
const PROJECT_FILE_ERROR: &str = "ProjectError";
const IO_ERROR: &str = "Io";
const BAD_LOCATION_ERROR: &str = "BadLocation";
const VALIDATION_ERROR: &str = "ValidationFailed";

#[derive(Debug, thiserror::Error)]
pub enum ProjectFileError {
  /// General Io workspace error
  #[error("{}::{} (err: {})", PROJECT_FILE_ERROR, IO_ERROR, .0)]
  Io(#[from] io::Error),
  /// An empty project name has been provided
  #[error("{}::{} (name: {})", PROJECT_FILE_ERROR, INVALID_PROJECT_NAME_ERROR, .0)]
  InvalidProjectName(String),
  /// A write serialization has failed
  #[error("{}::{} (err: {})", PROJECT_FILE_ERROR, WRITE_ERROR, .0)]
  Write(String),
  /// A read deserialization has failed
  #[error("{}::{} (err: {})", PROJECT_FILE_ERROR, READ_ERROR, .0)]
  Read(String),
  /// Invalid path, does not exist or cannot be written to
  #[error("{}::{} (path: {:?})", PROJECT_FILE_ERROR, BAD_LOCATION_ERROR, .0)]
  BadLocation(PathBuf),
  /// Validation errors
  #[error("{}::{} (errors: {:?})", PROJECT_FILE_ERROR, VALIDATION_ERROR, .0)]
  ValidationFailed(Vec<ProjectValidationError>)
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectValidationError {
  /// A Variable with an invalid name was found
  #[error("{} (name: {})", INVALID_VARIABLE_NAME_ERROR, .0)]
  InvalidVariableName(String),
  /// A Variable with an already existing name was found
  #[error("{} (name: {})", DUPLICATE_VARIABLE_NAME_ERROR, .0)]
  DuplicateVariableName(String),
  /// A Variable uses a non-existing profiles in its overrides
  #[error("{} (variable: {}, profile id: {})", INVALID_OVERRIDE_ERROR, .0, .1)]
  InvalidOverride(String, KId),
}
