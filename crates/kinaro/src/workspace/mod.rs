use crate::workspace::endpoint::WorkspaceEndpoint;
use crate::workspace::error::{ProjectError, Result, WorkspaceError};
use crate::workspace::test::WorkspaceTestsContainer;
use crate::workspace::variable::{WorkspaceProfile, WorkspaceVariable, WorkspaceVariables};
use chrono::{DateTime, Local};
use gpui::{Action, App, AsyncWindowContext, Entity, EventEmitter, SharedString, Window, actions};
use gpui::{AppContext, Context};
use gpui_component::WindowExt;
use gpui_component::notification::Notification;
use log::{debug, error, warn};
use project_file::{ProjectFile, ProjectFileError};
use serde::{Deserialize, Serialize};
use settings::GlobalSettings;
use std::collections::HashMap;
use std::fs;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub mod endpoint;
pub mod error;
pub mod test;
pub mod variable;

pub const WORKSPACES_FILENAME: &str = "workspaces.json";
pub const PROJECT_FILE_EXT: &str = "kpr";

#[derive(Serialize, Deserialize, Default)]
pub struct WorkspaceFile {
    active_project_id: Option<Uuid>,
    pub projects: Vec<WorkspaceProject>,
}

impl WorkspaceFile {
    fn load(file_path: &Path) -> Self {
        if file_path.exists() {
            fs::read(file_path)
                .map_err(|e| WorkspaceError::Io(e))
                .and_then(|file| {
                    serde_json::from_slice::<WorkspaceFile>(&file)
                        .map_err(|err| WorkspaceError::Read(err))
                })
                // If a project_error occurred while reading, create a new one
                .unwrap_or_default()
        } else {
            WorkspaceFile::default()
        }
    }
}

///// WORKSPACE ACTIONS /////
actions!(workspace, [CreateProject, OpenProject]);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = workspace, no_json)]
pub struct RemoveProject(pub Uuid);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = workspace, no_json)]
pub struct SwitchActiveProject(pub Uuid);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = workspace, no_json)]
pub struct RenameProject(pub Uuid, pub SharedString);

///// WORKSPACE EVENTS /////
pub enum WorkspaceEvent {
    ProjectsChanged,
}

impl EventEmitter<WorkspaceEvent> for Workspace {}

pub struct Workspace {
    active_project_id: Option<Uuid>,
    pub projects: HashMap<Uuid, Entity<WorkspaceProject>>,
}

impl Workspace {
    pub fn init(cx: &mut Context<Self>) -> Self {
        let file_path = cx.read_global(|settings: &GlobalSettings, _| {
            settings.config_dir.join(WORKSPACES_FILENAME)
        });

        let WorkspaceFile {
            mut active_project_id,
            projects,
        } = WorkspaceFile::load(&file_path);

        let projects: HashMap<Uuid, Entity<WorkspaceProject>> = projects
            .into_iter()
            .map(|mut project| {
                project.load(cx);
                let project_id = project.id;
                // If active project and loading failed, reset active project
                if Some(project_id) == active_project_id && !project.is_loaded() {
                    active_project_id = None;
                }
                (project_id, cx.new(|_| project))
            })
            .collect();

        Workspace {
            active_project_id,
            projects,
        }
    }

    pub fn active_project(&self) -> Option<Entity<WorkspaceProject>> {
        if let Some(active_project_id) = self.active_project_id {
            self.projects.get(&active_project_id).cloned()
        } else {
            None
        }
    }

    pub fn active_project_id(&self) -> Option<Uuid> {
        self.active_project_id.clone()
    }

    pub fn switch_project(&mut self, project_id: Uuid, cx: &mut Context<Self>) -> Result<()> {
        let project = self
            .projects
            .get(&project_id)
            .cloned()
            .ok_or(WorkspaceError::from(ProjectError::UnknownProject(
                project_id,
            )))?;
        let result = match project.read(cx).get_status() {
            WorkspaceProjectDataStatus::Unloaded => {
                Err(WorkspaceError::General("Unknown".to_string()))
            }
            WorkspaceProjectDataStatus::Loaded(_) => {
                self.active_project_id = Some(project_id);
                cx.emit(WorkspaceEvent::ProjectsChanged);
                Ok(())
            }
            WorkspaceProjectDataStatus::Moved => Err(WorkspaceError::from(
                ProjectError::BadLocation(project.read_with(cx, |project, _| project.path.clone())),
            )),
            WorkspaceProjectDataStatus::ExternallyModified => {
                Err(WorkspaceError::Project(ProjectError::ExternallyModified))
            }
            WorkspaceProjectDataStatus::LoadError(_) => {
                Err(WorkspaceError::Project(ProjectError::Invalid))
            }
        };
        if result.is_ok() {
            self.save(cx);
            cx.emit(WorkspaceEvent::ProjectsChanged);
        }
        result
    }

    pub fn switch_profile(&mut self, profile_id: Option<Uuid>, cx: &mut Context<Self>) {
        if let Some(active_project) = self.active_project() {
            active_project.update(cx, |project, cx| {
                project.active_profile = profile_id;
                cx.emit(WorkspaceProjectEvent::ProfilesModified);
            });

            self.save(cx);
        }
    }

    pub fn remove_project(&mut self, project_id: Uuid, cx: &mut Context<Self>) {
        self.projects.remove(&project_id);

        if self.active_project_id == Some(project_id) {
            self.active_project_id = None;
        }

        self.save(cx);

        cx.emit(WorkspaceEvent::ProjectsChanged);
    }

    pub fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) -> Result<()> {
        if let Some(project) = self.projects.values().find(|p| p.read(cx).path == path) {
            return self.switch_project(project.read(cx).id, cx);
        }
        let project = WorkspaceProject::open(path)?;
        let project_id = project.id;
        self.active_project_id = Some(project_id);
        self.projects.insert(project_id, cx.new(|_| project));

        self.save(cx);

        cx.emit(WorkspaceEvent::ProjectsChanged);

        Ok(())
    }

    pub fn create_project(
        &mut self,
        name: String,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if let Some(project) = self.projects.values().find(|p| p.read(cx).path == path) {
            return self.switch_project(project.read(cx).id, cx);
        }
        let project = cx.new(|_| WorkspaceProject::new(path, name));
        let project_id = project.read(cx).id;
        self.active_project_id = Some(project_id);
        self.projects.insert(project_id, project);

        self.save(cx);

        cx.emit(WorkspaceEvent::ProjectsChanged);

        Ok(())
    }

    fn save(&self, cx: &mut Context<Self>) {
        let config_dir = cx.read_global(|settings: &GlobalSettings, _| settings.config_dir.clone());
        let file_path = config_dir.join(WORKSPACES_FILENAME);

        cx.spawn(async move |this, cx| {
            _ = this.read_with(cx, |this, cx| {
                if let Err(err) = fs::create_dir_all(&*config_dir)
                    .map_err(|e| WorkspaceError::Io(e))
                    .and_then(|_| fs::File::create(&file_path).map_err(|e| WorkspaceError::Io(e)))
                    .and_then(|file| {
                        let file_content = this.to_file(cx);
                        serde_json::to_writer_pretty(file, &file_content)
                            .map_err(|err| WorkspaceError::Write(err))
                    })
                {
                    error!("Error while saving workspace file: {:?}", err);
                } else {
                    debug!("Saving workspace to {}", file_path.to_string_lossy());
                }
            });
        })
        .detach();
    }

    fn to_file(&self, cx: &App) -> WorkspaceFile {
        WorkspaceFile {
            active_project_id: self.active_project_id.clone(),
            projects: self
                .projects
                .iter()
                .map(|(_, p)| p.read(cx).clone())
                .collect(),
        }
    }
}

///// WORKSPACE PROJECTS ACTIONS /////
actions!(project, [ShowProfilesPanel]);

///// WORKSPACE PROJECT EVENTS /////
pub enum WorkspaceProjectEvent {
    ProfilesModified,
}

impl EventEmitter<WorkspaceProjectEvent> for WorkspaceProject {}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceProject {
    pub path: PathBuf,
    pub id: Uuid,
    pub name: SharedString,
    pub version: u16,
    pub created: DateTime<Local>,
    pub modified: DateTime<Local>,
    active_profile: Option<Uuid>,
    #[serde(skip)]
    data_status: WorkspaceProjectDataStatus,
}

impl Clone for WorkspaceProject {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            id: self.id,
            name: self.name.clone(),
            version: self.version,
            created: self.created,
            modified: self.modified,
            active_profile: self.active_profile.clone(),
            data_status: Default::default(),
        }
    }
}

impl PartialEq for WorkspaceProject {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl WorkspaceProject {
    #[inline]
    pub fn active_profile(&self) -> Option<Uuid> {
        self.active_profile.clone()
    }

    #[inline]
    pub fn is_loaded(&self) -> bool {
        matches!(self.data_status, WorkspaceProjectDataStatus::Loaded(_))
    }

    #[inline]
    pub fn get_status(&self) -> &WorkspaceProjectDataStatus {
        &self.data_status
    }

    /// Returns a reference to the data held by the project.
    /// Care should be taken to check the project status as this method will panic if the project is not loaded
    pub fn data(&self) -> &WorkspaceProjectData {
        match &self.data_status {
            WorkspaceProjectDataStatus::Loaded(data) => data,
            _ => panic!("Tried to read an unloaded project"),
        }
    }

    /// Returns a reference to the data held by the project.
    /// Care should be taken to check the project status as this method will panic if the project is not loaded
    pub fn data_mut(&mut self) -> &mut WorkspaceProjectData {
        match &mut self.data_status {
            WorkspaceProjectDataStatus::Loaded(data) => data,
            _ => panic!("Tried to read an unloaded project"),
        }
    }

    pub fn update_variables_and_profiles(
        &mut self,
        profiles: Vec<WorkspaceProfile>,
        variables: Vec<WorkspaceVariable>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let project_data = self.data_mut();
        if project_data.variables.update_variables(variables)
            || project_data.variables.update_profiles(profiles)
        {
            cx.emit(WorkspaceProjectEvent::ProfilesModified);
            self.save(window, cx);
        }
    }

    fn load(&mut self, cx: &mut App) {
        if !self.path.exists() {
            warn!("Could not find project at: {}", self.path.to_string_lossy());
            self.data_status = WorkspaceProjectDataStatus::Moved;
            return;
        }

        match ProjectFile::load(&self.path) {
            Ok(project_file) => {
                if project_file.id != self.id {
                    warn!(
                        "Project id mismatch for project at: {}. Workspace id: {}, file id: {}",
                        self.path.to_string_lossy(),
                        self.id,
                        project_file.id
                    );
                    self.data_status = WorkspaceProjectDataStatus::ExternallyModified;
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
                    self.name = SharedString::new(&project_file.name);
                }
                self.created = project_file.created;
                self.modified = project_file.modified;
                self.data_status = WorkspaceProjectDataStatus::Loaded(project)
            }
            Err(err) => {
                error!(
                    "Error loading project at: {}. Error: {}",
                    self.path.to_string_lossy(),
                    err
                );
                self.data_status = WorkspaceProjectDataStatus::LoadError(err);
            }
        };
    }

    fn open(path: PathBuf) -> Result<Self> {
        match ProjectFile::load(&path) {
            Ok(project) => {
                let data = WorkspaceProjectData::from_file(&project);
                Ok(Self {
                    path,
                    id: project.id,
                    name: SharedString::new(project.name),
                    version: 1,
                    created: project.created,
                    modified: project.modified,
                    active_profile: None,
                    data_status: WorkspaceProjectDataStatus::Loaded(data),
                })
            }
            Err(err) => Err(WorkspaceError::from(err)),
        }
    }

    fn new(path: PathBuf, name: String) -> Self {
        Self {
            path,
            id: Uuid::new_v4(),
            name: SharedString::new(name),
            version: 1,
            created: Default::default(),
            modified: Default::default(),
            active_profile: None,
            data_status: WorkspaceProjectDataStatus::Loaded(WorkspaceProjectData::new()),
        }
    }

    pub fn save(&mut self, window: &mut Window, cx: &Context<Self>) {
        if !self.is_loaded() {
            return;
        }
        cx.spawn_in(window, async move |this, cx| {
            if let Some(this) = this.upgrade() {
                if let Err(err) = this.update(cx, |this, cx| {
                    let project_file = this.to_file();
                    project_file
                        .save(&this.path)
                        .map_err(|e| WorkspaceError::from(e))?;
                    // Update the 'modified' attribute if save was successful
                    this.modified = project_file.modified;
                    Ok::<(), WorkspaceError>(())
                }) {
                    _ = cx.update(|window, cx| {
                        window.push_notification(Notification::error(format!("{}", err)), cx)
                    });
                }
            }
        })
        .detach();
    }

    fn to_file(&self) -> ProjectFile {
        let data = self.data();
        let variables = data.variables.to_file();
        let endpoints = data.endpoints.iter().map(|e| e.to_file()).collect();
        let tests = data.tests.to_file();
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
    Loaded(WorkspaceProjectData),
    Moved,
    ExternallyModified,
    LoadError(ProjectFileError),
}

impl Default for WorkspaceProjectDataStatus {
    fn default() -> Self {
        WorkspaceProjectDataStatus::Unloaded
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceProjectData {
    pub variables: WorkspaceVariables,
    pub endpoints: Vec<WorkspaceEndpoint>,
    pub tests: WorkspaceTestsContainer,
}

impl WorkspaceProjectData {
    fn new() -> Self {
        Self {
            variables: Default::default(),
            endpoints: vec![],
            tests: Default::default(),
        }
    }

    fn from_file(project_file: &ProjectFile) -> WorkspaceProjectData {
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
