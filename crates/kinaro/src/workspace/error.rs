use gpui_component::notification::Notification;
use ki_project::ProjectFileError;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

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
}

impl Into<Notification> for WorkspaceError {
    fn into(self) -> Notification {
        match self {
            WorkspaceError::General(message) => Notification::error(message),
            WorkspaceError::Io(err) => Notification::error(format!("IO Error: {}", err)),
            WorkspaceError::Write(err) => {
                Notification::error(format!("Could not write workspace file: {}", err))
            }
            WorkspaceError::Read(err) => {
                Notification::error(format!("Could not read workspace file: {}", err))
            }
            WorkspaceError::Project(err) => err.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// General Io workspace error
    #[error("ProjectError::Io (err: {})", .0)]
    Io(#[from] io::Error),
    /// A Json serialization has failed
    #[error("ProjectError::Write (err: {:?})", .0)]
    Write(String),
    /// A Json deserialization has failed
    #[error("ProjectError::Read (err: {:?})", .0)]
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
    /// The project has invalid name
    #[error("ProjectError::InvalidName (name: {})", .0)]
    InvalidName(String),
    /// An operation was attempted on a missing profile
    #[error("ProjectError::ProfileNotFound")]
    ProfileNotFound,
}

impl From<ProjectFileError> for ProjectError {
    fn from(value: ProjectFileError) -> Self {
        match value {
            ProjectFileError::Io(err) => ProjectError::Io(err),
            ProjectFileError::InvalidName(name) => ProjectError::InvalidName(name),
            ProjectFileError::WriteYaml(err) => ProjectError::Write(err),
            ProjectFileError::ReadYaml(err) => ProjectError::Read(err),
            ProjectFileError::BadLocation(path) => ProjectError::BadLocation(path),
        }
    }
}

impl Into<Notification> for ProjectError {
    fn into(self) -> Notification {
        match self {
            ProjectError::Write(err) => {
                Notification::error(format!("Could not write project file: {}", err))
            }
            ProjectError::Read(err) => {
                Notification::error(format!("Could not read project file: {}", err))
            }
            ProjectError::BadLocation(path) => Notification::error(format!(
                "Invalid location for project file: {}",
                path.to_string_lossy()
            )),
            ProjectError::UnknownProject(id) => {
                Notification::error(format!("The project with id: {} is unknown", id))
            }
            ProjectError::ExternallyModified => Notification::error(
                "The project has been externally modified, remove from workspace and add it again",
            ),
            ProjectError::Invalid => Notification::error("The project could not be loaded"),
            ProjectError::InvalidName(name) => {
                Notification::error(format!("Invalid name for project: {}", name))
            }
            ProjectError::ProfileNotFound => Notification::warning("Unknown profile"),
            ProjectError::Io(err) => {
                Notification::error(format!("I/O error on project file: {}", err))
            }
        }
    }
}
