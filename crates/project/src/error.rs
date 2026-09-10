use std::io;
use std::path::PathBuf;

/// Standard result for Project plugin
pub type Result<T> = std::result::Result<T, ProjectFileError>;

/// GLOBAL ERROR TYPES
const WRITE_ERROR: &str = "Write";
const READ_ERROR: &str = "Read";
const INVALID_NAME_ERROR: &str = "InvalidName";

/// PROJECT ERROR TYPES
const PROJECT_FILE_ERROR: &str = "ProjectError";
const IO_ERROR: &str = "Io";
const BAD_LOCATION_ERROR: &str = "BadLocation";

#[derive(Debug, thiserror::Error)]
pub enum ProjectFileError {
  /// General Io workspace error
  #[error("{}::{} (err: {})", PROJECT_FILE_ERROR, IO_ERROR, .0)]
  Io(#[from] io::Error),
  /// An empty name has been provided
  #[error("{}::{} (name: {})", PROJECT_FILE_ERROR, INVALID_NAME_ERROR, .0)]
  InvalidName(String),
  /// A wtite serialization has failed
  #[error("{}::{} (err: {:?})", PROJECT_FILE_ERROR, WRITE_ERROR, .0)]
  WriteYaml(String),
  /// A read deserialization has failed
  #[error("{}::{} (err: {:?})", PROJECT_FILE_ERROR, READ_ERROR, .0)]
  ReadYaml(String),
  /// Invalid path, does not exist or cannot be written to
  #[error("{}::{} (path: {:?})", PROJECT_FILE_ERROR, BAD_LOCATION_ERROR, .0)]
  BadLocation(PathBuf),
}
