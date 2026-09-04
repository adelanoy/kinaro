use gpui_component::notification::Notification;
use ki_project::ProjectFileError;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// An operation tried to access a project that failed to load
    #[error("WorkspaceError::ProjectNotLoaded")]
    ProjectNotLoaded,
    /// General Io workspace error
    #[error("WorkspaceError::Io (err: {})", .0)]
    Io(String),
    /// A JSON serialization has failed
    #[error("WorkspaceError::WriteJson (err: {:?})", .0)]
    Write(serde_json::Error),
    /// A JSON deserialization has failed
    #[error("WorkspaceError::ReadJson (err: {:?})", .0)]
    Read(serde_json::Error),
    /// General project errors
    #[error("WorkspaceError::{})", .0)]
    Project(#[from] ProjectError),
}

impl From<WorkspaceError> for Notification {
    fn from(value: WorkspaceError) -> Notification {
        match value {
            WorkspaceError::ProjectNotLoaded => Notification::warning("The project is invalid"),
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

#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectError {
    /// General Io workspace error
    #[error("ProjectError::Io (err: {})", .0)]
    Io(String),
    /// A write serialization has failed
    #[error("ProjectError::Write (err: {:?})", .0)]
    Write(String),
    /// A read deserialization has failed
    #[error("ProjectError::Read (err: {:?})", .0)]
    Read(String),
    /// A project could not be written or read to this invalid location
    #[error("ProjectError::BadLocation (path: {:?})", .0)]
    BadLocation(PathBuf),
    /// Tried to access an unknown project
    #[error("ProjectError::UnknownProject (id: {:?})", .0)]
    UnknownProject(PathBuf),
    /// The project has invalid name
    #[error("ProjectError::InvalidName (name: {})", .0)]
    InvalidName(String),
    /// An operation was attempted on a missing profile
    #[error("ProjectError::ProfileNotFound")]
    ProfileNotFound,
    /// An operation was attempted on a missing variable
    #[error("ProjectError::VariableNotFound")]
    VariableNotFound,
    /// An operation was attempted on a missing test
    #[error("ProjectError::TestNotFound")]
    TestNotFound,
}

impl From<ProjectFileError> for ProjectError {
    fn from(value: ProjectFileError) -> Self {
        match value {
            ProjectFileError::Io(err) => ProjectError::Io(err.to_string()),
            ProjectFileError::InvalidName(name) => ProjectError::InvalidName(name),
            ProjectFileError::WriteYaml(err) => ProjectError::Write(err),
            ProjectFileError::ReadYaml(err) => ProjectError::Read(err),
            ProjectFileError::BadLocation(path) => ProjectError::BadLocation(path),
        }
    }
}

impl From<ProjectError> for Notification {
    fn from(value: ProjectError) -> Notification {
        match value {
            ProjectError::Io(err) => {
                Notification::error(format!("I/O error on project file: {}", err))
            }
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
            ProjectError::UnknownProject(path) => {
                Notification::error(format!("The project with id: {:?} is unknown", path))
            }
            ProjectError::InvalidName(name) => {
                Notification::error(format!("Invalid name for project: {}", name))
            }
            ProjectError::ProfileNotFound => Notification::warning("Unknown profile"),
            ProjectError::VariableNotFound => Notification::warning("Unknown variable"),
            ProjectError::TestNotFound => Notification::warning("The test could not be found"),
        }
    }
}
