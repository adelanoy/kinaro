use std::io;
use std::path::PathBuf;

/// Standard result for Project plugin
pub type Result<T> = std::result::Result<T, SettingsError>;

/// GLOBAL ERROR TYPES
const WRITE_ERROR: &str = "Write";
const READ_ERROR: &str = "Read";

/// PROJECT ERROR TYPES
const SETTINGS_ERROR: &str = "ProjectError";
const IO_ERROR: &str = "Io";
const BAD_LOCATION_ERROR: &str = "BadLocation";

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
  /// General Io error
  #[error("{}::{} (err: {})", SETTINGS_ERROR, IO_ERROR, .0)]
  Io(#[from] io::Error),
  /// A JSON serialization has failed
  #[error("{}::{} (err: {:?})", SETTINGS_ERROR, WRITE_ERROR, .0)]
  WriteJson(String),
  /// A JSON deserialization has failed
  #[error("{}::{} (err: {:?})", SETTINGS_ERROR, READ_ERROR, .0)]
  ReadJson(String),
  /// Invalid path, does not exist or cannot be written to
  #[error("{}::{} (path: {:?})", SETTINGS_ERROR, BAD_LOCATION_ERROR, .0)]
  BadLocation(PathBuf),
}
