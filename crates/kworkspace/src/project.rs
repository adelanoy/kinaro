use crate::endpoint::WorkspaceEndpoint;
use crate::error::ProjectError::Invalid;
use crate::project::WorkspaceProjectDataStatus::{LoadError, Loaded, Unloaded};
use crate::test::WorkspaceTestsContainer;
use crate::variable::WorkspaceVariables;
use crate::{Result, WorkspaceError};
use chrono::{DateTime, Local};
use project_file::{ProjectFile, ProjectFileError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use WorkspaceProjectDataStatus::ExternallyModified;

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceProject {
    pub path: PathBuf,
    pub id: Uuid,
    pub name: String,
    pub version: u16,
    pub created: DateTime<Local>,
    pub modified: DateTime<Local>,
    pub active_profile: Option<Uuid>,
    #[serde(skip)]
    data_status: WorkspaceProjectDataStatus,
}

impl PartialEq for WorkspaceProject {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl WorkspaceProject {
    pub(crate) fn load(&mut self) {
        if !self.path.exists() {
            self.data_status = WorkspaceProjectDataStatus::Moved;
            return;
        }

        match ProjectFile::load(&self.path) {
            Ok(project_file) => {
                if project_file.id != self.id {
                    self.data_status = ExternallyModified;
                    return;
                }
                let project = WorkspaceProjectData::from_file(&project_file);
                // Update the ProjectInfo with the file data (in case of modification done externally without any major impact)
                if let Some(active_profile) = &self.active_profile {
                    if !project
                        .variables
                        .profiles
                        .iter()
                        .any(|p| p.id == *active_profile)
                    {
                        self.active_profile = None;
                    }
                }
                if self.name != project_file.name {
                    self.name.clone_from(&project_file.name);
                }
                self.created = project_file.created;
                self.modified = project_file.modified;
                self.data_status = Loaded(project)
            }
            Err(err) => {
                self.data_status = LoadError(err);
                return;
            }
        };
    }

    pub(crate) fn open(path: PathBuf) -> Result<Self> {
        match ProjectFile::load(&path) {
            Ok(project) => {
                let data = WorkspaceProjectData::from_file(&project);
                Ok(Self {
                    path,
                    id: project.id,
                    name: project.name,
                    version: 1,
                    created: project.created,
                    modified: project.modified,
                    active_profile: None,
                    data_status: Loaded(data),
                })
            }
            Err(err) => Err(WorkspaceError::from(err)),
        }
    }

    pub(crate) fn create(path: PathBuf, name: String) -> Result<Self> {
        let mut workspace_project = Self {
            path,
            id: Uuid::new_v4(),
            name,
            version: 1,
            created: Default::default(),
            modified: Default::default(),
            active_profile: None,
            data_status: Loaded(Default::default()),
        };
        workspace_project.save()?;
        Ok(workspace_project)
    }

    pub(crate) fn save(&mut self) -> Result<()> {
        if !self.is_loaded() {
            return Err(WorkspaceError::Project(Invalid));
        }
        let project_file = self.get_file();
        project_file
            .save(&self.path)
            .map_err(|e| WorkspaceError::from(e))?;
        // Update the 'modified' attribute if save was successful
        self.modified = project_file.modified;

        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self.data_status, Loaded(_))
    }

    pub fn get_status(&self) -> &WorkspaceProjectDataStatus {
        &self.data_status
    }

    ///
    /// Returns a reference to the data held by the project.
    /// Care should be taken to check the project status as this method will panic if the project is not loaded
    pub fn data(&self) -> &WorkspaceProjectData {
        match &self.data_status {
            Loaded(data) => data,
            _ => panic!("Tried to read an unloaded project"),
        }
    }

    /// Returns a mut reference to the data held by the project
    /// Care should be taken to check the project status as this method will panic if the project is not loaded
    pub fn data_mut(&mut self) -> &mut WorkspaceProjectData {
        match &mut self.data_status {
            Loaded(data) => data,
            _ => panic!("Tried to unwrap an unloaded project"),
        }
    }

    fn get_file(&self) -> ProjectFile {
        let data = self.data();
        let variables = data.variables.get_file();
        let endpoints = data.endpoints.iter().map(|e| e.get_file()).collect();
        let tests = data.tests.get_file();
        let modified = Local::now();

        ProjectFile {
            name: self.name.clone(),
            id: self.id,
            version: self.version,
            created: self.created,
            modified,
            variables,
            endpoints,
            tests,
        }
    }
}

#[derive(Debug)]
pub enum WorkspaceProjectDataStatus {
    Unloaded,
    Loaded(WorkspaceProjectData),
    Moved,
    ExternallyModified,
    LoadError(ProjectFileError),
}

impl Default for WorkspaceProjectDataStatus {
    fn default() -> Self {
        Unloaded
    }
}

#[derive(Clone, Debug, Default)]
pub struct WorkspaceProjectData {
    pub variables: WorkspaceVariables,
    pub endpoints: Vec<WorkspaceEndpoint>,
    pub tests: WorkspaceTestsContainer,
}

impl WorkspaceProjectData {
    pub fn from_file(project_file: &ProjectFile) -> WorkspaceProjectData {
        let variables = WorkspaceVariables::from_file(&project_file.variables);
        let endpoints = WorkspaceEndpoint::from_file(&project_file.endpoints);
        let tests = WorkspaceTestsContainer::from_file(&project_file.tests);

        Self {
            variables,
            endpoints,
            tests,
        }
    }
}
