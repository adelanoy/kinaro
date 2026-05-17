use crate::workspace::endpoint::WorkspaceEndpoint;
use crate::workspace::error::{ProjectError, Result, WorkspaceError};
use crate::workspace::test::WorkspaceTestsContainer;
use crate::workspace::variable::WorkspaceVariables;
use chrono::{DateTime, Local};
use gpui::{App, Entity, SharedString};
use gpui::{AppContext, Context};
use project_file::{ProjectFile, ProjectFileError};
use serde::{Deserialize, Serialize};
use settings::GlobalSettings;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub(crate) mod endpoint;
pub(crate) mod error;
pub(crate) mod test;
pub(crate) mod variable;

pub const WORKSPACES_FILENAME: &str = "workspaces.json";

#[derive(Serialize, Deserialize, Default)]
pub struct Workspace {
    active_project: Option<Uuid>,
    pub projects: Vec<WorkspaceProject>,
}

impl Workspace {
    pub fn init(cx: &mut Context<Self>) -> Self {
        let file_path = cx.read_global(|settings: &GlobalSettings, _| {
            settings.config_dir.join(WORKSPACES_FILENAME)
        });

        let mut workspace = if file_path.exists() {
            fs::read(&file_path)
                .map_err(|e| WorkspaceError::Io(e))
                .and_then(|file| {
                    serde_json::from_slice::<Workspace>(&file)
                        .map_err(|err| WorkspaceError::Read(err))
                })
                // If a project_error occurred while reading, create a new one
                .unwrap_or_default()
        } else {
            Workspace::default()
        };
        workspace.projects.iter_mut().for_each(|p| p.load(cx));

        workspace
    }

    pub fn save(&self, cx: &mut App) -> Result<()> {
        let config_dir = cx.read_global(|settings: &GlobalSettings, _| settings.config_dir.clone());
        let file_path = config_dir.join(WORKSPACES_FILENAME);

        fs::create_dir_all(&*config_dir)
            .map_err(|e| WorkspaceError::Io(e))
            .and_then(|_| fs::File::create(&file_path).map_err(|e| WorkspaceError::Io(e)))
            .and_then(|file| {
                serde_json::to_writer_pretty(file, self).map_err(|err| WorkspaceError::Write(err))
            })?;
        Ok(())
    }

    pub fn get_active_project(&self) -> Option<&WorkspaceProject> {
        if let Some(active_project) = self.active_project {
            self.projects
                .iter()
                .find(|p| p.id == active_project && p.is_loaded())
        } else {
            None
        }
    }

    pub fn get_active_project_id(&self) -> Option<Uuid> {
        self.active_project.clone()
    }

    pub fn get_active_project_mut(&mut self) -> Option<&mut WorkspaceProject> {
        if let Some(active_project) = self.active_project {
            self.projects
                .iter_mut()
                .find(|p| p.id == active_project && p.is_loaded())
        } else {
            None
        }
    }

    pub fn switch_project(&mut self, project_id: Uuid, cx: &mut App) -> Result<()> {
        self.save_active_project(cx)?;

        let info = self
            .projects
            .iter_mut()
            .find(|p| p.id == project_id)
            .ok_or(WorkspaceError::from(ProjectError::UnknownProject(
                project_id,
            )))?;
        match info.get_status() {
            WorkspaceProjectDataStatus::Unloaded => {
                Err(WorkspaceError::General("Unknown".to_string()))
            }
            WorkspaceProjectDataStatus::Loaded(_) => {
                self.active_project = Some(project_id);
                Ok(())
            }
            WorkspaceProjectDataStatus::Moved => Err(WorkspaceError::from(
                ProjectError::BadLocation(info.path.clone()),
            )),
            WorkspaceProjectDataStatus::ExternallyModified => {
                Err(WorkspaceError::Project(ProjectError::ExternallyModified))
            }
            WorkspaceProjectDataStatus::LoadError(_) => {
                Err(WorkspaceError::Project(ProjectError::Invalid))
            }
        }
    }

    pub fn remove_project(&mut self, project_id: Uuid, cx: &App) {
        let project_pos = self
            .projects
            .iter()
            .position(|p| p.id == project_id)
            .unwrap();
        let mut project = self.projects.remove(project_pos);
        let _ = project.save(cx);

        // If the project was active, switch to the next project, if any
        if self.active_project == Some(project.id) {
            self.active_project = self.projects.iter().next().map_or(None, |p| Some(p.id));
        }
    }

    pub fn open_project(&mut self, path: PathBuf, cx: &mut App) -> Result<()> {
        if let Some(pos) = self.projects.iter().position(|p| p.path == path) {
            return self.switch_project(self.projects[pos].id, cx);
        }
        let project = WorkspaceProject::open(path, cx)?;
        self.active_project = Some(project.id);
        self.projects.push(project);

        Ok(())
    }

    pub fn create_project(&mut self, name: String, path: PathBuf, cx: &mut App) -> Result<()> {
        if let Some(pos) = self.projects.iter().position(|p| p.path == path) {
            return self.switch_project(self.projects[pos].id, cx);
        }
        let project = WorkspaceProject::create(path, name, cx)?;
        self.active_project = Some(project.id);
        self.projects.push(project);

        Ok(())
    }

    fn save_active_project(&mut self, cx: &App) -> Result<()> {
        if let Some(project) = self.get_active_project_mut() {
            if project.is_loaded() {
                project.save(cx)
            } else {
                Err(WorkspaceError::Project(ProjectError::Invalid))
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceProject {
    pub path: PathBuf,
    pub id: Uuid,
    pub name: SharedString,
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
    pub(crate) fn load(&mut self, cx: &mut App) {
        if !self.path.exists() {
            self.data_status = WorkspaceProjectDataStatus::Moved;
            return;
        }

        match ProjectFile::load(&self.path) {
            Ok(project_file) => {
                if project_file.id != self.id {
                    self.data_status = WorkspaceProjectDataStatus::ExternallyModified;
                }
                let project = WorkspaceProjectData::from_file(&project_file, cx);
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
                    self.name = SharedString::new(&project_file.name);
                }
                self.created = project_file.created;
                self.modified = project_file.modified;
                self.data_status = WorkspaceProjectDataStatus::Loaded(cx.new(|_| project))
            }
            Err(err) => {
                self.data_status = WorkspaceProjectDataStatus::LoadError(err);
            }
        };
    }

    pub(crate) fn open(path: PathBuf, cx: &mut App) -> Result<Self> {
        match ProjectFile::load(&path) {
            Ok(project) => {
                let data = WorkspaceProjectData::from_file(&project, cx);
                Ok(Self {
                    path,
                    id: project.id,
                    name: SharedString::new(project.name),
                    version: 1,
                    created: project.created,
                    modified: project.modified,
                    active_profile: None,
                    data_status: WorkspaceProjectDataStatus::Loaded(cx.new(|_| data)),
                })
            }
            Err(err) => Err(WorkspaceError::from(err)),
        }
    }

    pub(crate) fn create(path: PathBuf, name: String, cx: &mut App) -> Result<Self> {
        let mut workspace_project = Self {
            path,
            id: Uuid::new_v4(),
            name: SharedString::new(name),
            version: 1,
            created: Default::default(),
            modified: Default::default(),
            active_profile: None,
            data_status: WorkspaceProjectDataStatus::Loaded(cx.new(|_| Default::default())),
        };
        workspace_project.save(cx)?;
        Ok(workspace_project)
    }

    pub(crate) fn save(&mut self, cx: &App) -> Result<()> {
        if !self.is_loaded() {
            return Err(WorkspaceError::Project(ProjectError::Invalid));
        }
        let project_file = self.get_file(cx);
        project_file
            .save(&self.path)
            .map_err(|e| WorkspaceError::from(e))?;
        // Update the 'modified' attribute if save was successful
        self.modified = project_file.modified;

        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self.data_status, WorkspaceProjectDataStatus::Loaded(_))
    }

    pub fn get_status(&self) -> &WorkspaceProjectDataStatus {
        &self.data_status
    }

    /// Returns a reference to the data held by the project.
    /// Care should be taken to check the project status as this method will panic if the project is not loaded
    pub fn data(&self) -> &Entity<WorkspaceProjectData> {
        match &self.data_status {
            WorkspaceProjectDataStatus::Loaded(data) => data,
            _ => panic!("Tried to read an unloaded project"),
        }
    }

    fn get_file(&self, cx: &App) -> ProjectFile {
        let data = self.data().read(cx);
        let variables = data.variables.get_file();
        let endpoints = data.endpoints.iter().map(|e| e.get_file()).collect();
        let tests = data.tests.get_file();
        let modified = Local::now();

        ProjectFile {
            name: self.name.to_string(),
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
    Loaded(Entity<WorkspaceProjectData>),
    Moved,
    ExternallyModified,
    LoadError(ProjectFileError),
}

impl Default for WorkspaceProjectDataStatus {
    fn default() -> Self {
        WorkspaceProjectDataStatus::Unloaded
    }
}

#[derive(Clone, Debug, Default)]
pub struct WorkspaceProjectData {
    pub variables: WorkspaceVariables,
    pub endpoints: Vec<WorkspaceEndpoint>,
    pub tests: WorkspaceTestsContainer,
}

impl WorkspaceProjectData {
    pub fn from_file(project_file: &ProjectFile, _cx: &mut App) -> WorkspaceProjectData {
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
