use project_file::ProjectFileError;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

/// Standard result for Project plugin
pub type Result<T> = std::result::Result<T, WorkspaceError>;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// General workspace error
    #[error("WorkspaceError::General (err: {})", .0)]
    General(String),
    /// General Io workspace error
    #[error("WorkspaceError::Io (err: {})", .0)]
    Io(#[from] io::Error),
    /// A Json serialization has failed
    #[error("WorkspaceError::WriteJson (err: {:?})", .0)]
    Write(serde_json::Error),
    /// A Json deserialization has failed
    #[error("WorkspaceError::ReadJson (err: {:?})", .0)]
    Read(serde_json::Error),
    /// General project errors
    #[error("WorkspaceError::{})", .0)]
    Project(#[from] ProjectError),
    /// Variables & profiles project errors
    #[error("WorkspaceError::{})", .0)]
    Test(#[from] TestError),
}

impl From<ProjectFileError> for WorkspaceError {
    fn from(value: ProjectFileError) -> Self {
        match value {
            ProjectFileError::Io(err) => WorkspaceError::Io(err),
            ProjectFileError::InvalidName(name) => {
                WorkspaceError::Project(ProjectError::InvalidName(name))
            }
            ProjectFileError::WriteYaml(err) => WorkspaceError::Project(ProjectError::Write(err)),
            ProjectFileError::ReadYaml(err) => WorkspaceError::Project(ProjectError::Read(err)),
            ProjectFileError::BadLocation(path) => {
                WorkspaceError::Project(ProjectError::BadLocation(path))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// A Json serialization has failed
    #[error("WorkspaceError::Write (err: {:?})", .0)]
    Write(String),
    /// A Json deserialization has failed
    #[error("WorkspaceError::Read (err: {:?})", .0)]
    Read(String),
    /// A project could not be written or read to this invalid location
    #[error("ProjectError::BadLocation (path: {:?})", .0)]
    BadLocation(PathBuf),
    /// Tried to access an unknown project
    #[error("ProjectError::UnknownProject (id: {})", .0)]
    UnknownProject(Uuid),
    /// The project located at this path has a different ID then expected
    #[error("ProjectError::ExternallyModified")]
    ExternallyModified,
    /// The project has invalid data
    #[error("ProjectError::Invalid")]
    Invalid,
    /// The project has invalid data
    #[error("ProjectError::InvalidName (name: {})", .0)]
    InvalidName(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TestError {
    /// Tried to move a test to an invalid location
    #[error("TestError::InvalidMoveTarget")]
    InvalidMoveTarget,
    /// Tried to move a non-existing test
    #[error("TestError::InvalidMoveSource")]
    InvalidMoveSource,
}
