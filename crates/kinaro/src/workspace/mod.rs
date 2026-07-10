use crate::workspace::error::WorkspaceError::ProjectNotLoaded;
use crate::workspace::error::{ProjectError, WorkspaceError};
pub(crate) use crate::workspace::project::{Project, ProjectEvent};
use gpui::{actions, App, Entity, EventEmitter, SharedString, Subscription};
use gpui::{AppContext, Context};
use ki_settings::GlobalSettings;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub mod endpoint;
pub mod error;
pub mod project;
pub mod test;
pub mod variable;

const WORKSPACES_FILENAME: &str = "workspace.json";

/// Result alias for Workspace
pub type Result<T> = std::result::Result<T, WorkspaceError>;

/// Serialized version of a workspace
#[derive(Serialize, Deserialize, Default)]
struct WorkspaceFile {
    active_project: Option<PathBuf>,
    projects: Vec<FileProjectMetadata>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct FileProjectMetadata {
    /// The project's name
    name: String,
    /// The path where the project file is located. Also serves as a key
    path: PathBuf,
    /// The currently active profile id, if any
    active_profile: Option<Uuid>,
}

impl WorkspaceFile {
    fn load(file_path: &Path) -> Self {
        if file_path.exists() {
            fs::read(file_path)
                .map_err(|e| WorkspaceError::Io(e.to_string()))
                .and_then(|file| {
                    serde_json::from_slice::<WorkspaceFile>(&file).map_err(|err| {
                        error!("Could not read workspace file: {}\nResetting...", err);
                        WorkspaceError::Read(err)
                    })
                })
                // If a project_error occurred while reading, create a new one
                .unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

///// WORKSPACE ACTIONS /////
actions!(workspace, [CreateProject, OpenProject]);

///// WORKSPACE EVENTS /////
pub enum WorkspaceEvent {
    ProjectsChanged,
    ActiveProjectChanged,
}

/// The workspace is the structure that holds all projects loaded or not.
///
/// It is basically a [`HashMap`] with a [`ProjectMetadata`] as a key and the result of the loading (Either a [`Entity<Project>`] or a [`ProjectError`])
/// as value
pub struct Workspace {
    active_project: Option<PathBuf>,
    workspace_projects: HashMap<PathBuf, WorkspaceProject>,
}

impl Workspace {
    /// Reads the workspace file and load all projects. If a project could not be loaded, the metadata associated have a *valid: false* flag
    /// and the associated value is a [`ProjectError`], else, it is [`Entity<Project>`]
    ///
    /// Some metadata (such as the active profile) are refreshed after a successful load, in case they were externally modified
    ///
    /// If the active project path read from the workspace file points to a non-existant project file, or if the file failed to load, the active project is
    /// set to *None*
    pub fn init(cx: &mut Context<Self>) -> Self {
        let file_path = cx.read_global(|settings: &GlobalSettings, _| {
            settings.config_dir.join(WORKSPACES_FILENAME)
        });

        let WorkspaceFile {
            active_project,
            projects,
        } = WorkspaceFile::load(&file_path);

        let workspace_projects: HashMap<PathBuf, WorkspaceProject> = projects
            .into_iter()
            .map(
                |metadata| match Project::load(&metadata.path, metadata.active_profile, cx) {
                    Ok(project) => {
                        // Refresh the metadata now that the project is loaded, in case they were change externally
                        let path = project.read(cx).path.clone();
                        let _sub = cx.subscribe(&project, Self::on_project_event);
                        (
                            path.clone(),
                            WorkspaceProject::project(project, _sub, &metadata),
                        )
                    }
                    Err(err) => (
                        metadata.path.clone(),
                        WorkspaceProject::error(err, metadata),
                    ),
                },
            )
            .collect();

        // Update the active_project, in case it points to an unloaded/moved project
        let active_project = match active_project {
            None => None,
            Some(path) => match workspace_projects.get(&path) {
                None => None,
                Some(workspace_project) => {
                    if workspace_project.is_loaded() {
                        Some(path)
                    } else {
                        None
                    }
                }
            },
        };

        Workspace {
            active_project,
            workspace_projects,
        }
    }

    pub fn all_project_infos(&self) -> Vec<WorkspaceProjectInfo> {
        self.workspace_projects
            .iter()
            .map(|(path, workspace_project)| {
                let path = path.clone();
                let name = workspace_project.name.clone();
                let loaded = workspace_project.is_loaded();
                let active = Some(&path) == self.active_project.as_ref();
                WorkspaceProjectInfo {
                    name,
                    path,
                    loaded,
                    active,
                }
            })
            .collect()
    }

    /// Returns a summary of project that failed to load
    pub fn all_failed_projects(&self) -> Vec<(SharedString, PathBuf, String)> {
        let mut summary = Vec::new();

        for (path, wp) in &self.workspace_projects {
            match &wp.data {
                WorkspaceProjectData::Error(err, _) => {
                    summary.push((wp.name.clone(), path.clone(), err.to_string()))
                }
                WorkspaceProjectData::Loaded { .. } => continue,
            }
        }

        summary
    }

    /// Returns the currently active project
    ///
    /// If no active project is set or if it refers to a project that failed to load at init, returns *None*
    pub fn active_project(&self) -> Option<Entity<Project>> {
        if let Some(active_project) = &self.active_project {
            let workspace_project = self.workspace_projects.get(active_project)?;
            match &workspace_project.data {
                WorkspaceProjectData::Loaded { project, .. } => Some(project.clone()),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Creates a new project with the given name, and saved at the given path and append it to the workspace
    ///
    /// If the project's path is already in the workspace, does nothing
    ///
    /// In both cases, the project is set as the currently active project
    /// # Events
    /// Emits a [`WorkspaceEvent::ProjectsChanged`] if the project is added to the workspace
    ///
    /// Emits a [`WorkspaceEvent::ActiveProjectChanged`] when the project is set as active
    pub fn create_project(&mut self, name: SharedString, path: PathBuf, cx: &mut Context<Self>) {
        if !self.workspace_projects.contains_key(&path) {
            let project = cx.new(|cx| Project::new(path.clone(), name.clone(), cx));
            let _sub = cx.subscribe(&project, Self::on_project_event);
            let workspace_project = WorkspaceProject {
                name,
                data: WorkspaceProjectData::Loaded { project, _sub },
            };
            self.workspace_projects
                .insert(path.clone(), workspace_project);
            cx.emit(WorkspaceEvent::ProjectsChanged);
        }

        self.active_project = Some(path.clone());
        cx.emit(WorkspaceEvent::ActiveProjectChanged);
        self.save(cx);
    }

    /// Opens an existing project and append it to the workspace
    ///
    /// If the project is already in the workspace, does nothing
    ///
    /// In both cases, the project is set as the currently active project
    /// # Events
    /// Emits a [`WorkspaceEvent::ProjectsChanged`] if the project is added to the workspace
    ///
    /// Emits a [`WorkspaceEvent::ActiveProjectChanged`] when the project is set as active
    /// # Returns
    /// If the project failed to load, returns a [`WorkspaceError::Project`]
    pub fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) -> Result<()> {
        if !self.workspace_projects.contains_key(&path) {
            let project = Project::load(&path, None, cx)?;
            let name = project.read(cx).name.clone();
            let _sub = cx.subscribe(&project, Self::on_project_event);
            let workspace_project = WorkspaceProject {
                name,
                data: WorkspaceProjectData::Loaded { project, _sub },
            };
            self.workspace_projects
                .insert(path.clone(), workspace_project);
            cx.emit(WorkspaceEvent::ProjectsChanged);
        }

        self.active_project = Some(path.clone());
        cx.emit(WorkspaceEvent::ActiveProjectChanged);
        self.save(cx);
        Ok(())
    }

    /// Attempts to reload an unloaded project
    ///
    /// # Events
    /// Emits a [`WorkspaceEvent::ProjectsChanged`] if the project is successfully removed from the workspace
    pub fn reload_project(
        &mut self,
        project_path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Result<bool> {
        let updated_wp;
        {
            let Some(project) = self.workspace_projects.get_mut(&project_path) else {
                return Ok(false);
            };

            updated_wp = match &project.data {
                WorkspaceProjectData::Error(_, metadata) => {
                    match Project::load(&project_path, metadata.active_profile, cx) {
                        Ok(project) => {
                            let _sub = cx.subscribe(&project, Self::on_project_event);
                            Some(WorkspaceProject::project(project, _sub, metadata))
                        }
                        Err(err) => Some(WorkspaceProject::error(err, metadata.clone())),
                    }
                }
                WorkspaceProjectData::Loaded { .. } => None,
            };
        }

        if updated_wp.is_some() {
            let updated_wp = updated_wp.unwrap();
            let result: Result<bool> = match &updated_wp.data {
                WorkspaceProjectData::Error(err, _) => {
                    println!("err: {}", err.to_string());
                    Err(err.clone().into())
                }
                WorkspaceProjectData::Loaded { .. } => Ok(true),
            };
            self.workspace_projects
                .insert(project_path.clone(), updated_wp);
            cx.emit(WorkspaceEvent::ProjectsChanged);
            if result.is_ok() {
                self.active_project = Some(project_path);
                cx.emit(WorkspaceEvent::ActiveProjectChanged);
            }
            self.save(cx);
            result
        } else {
            Ok(false)
        }
    }

    /// Renames a project
    ///
    /// # Events
    /// Emits a [`WorkspaceEvent::ProjectsChanged`] if the project is successfully removed from the workspace
    pub fn rename_project(
        &mut self,
        project_path: &PathBuf,
        new_name: SharedString,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.workspace_projects.get_mut(project_path) else {
            return;
        };

        project.name = new_name.clone();
        match &project.data {
            WorkspaceProjectData::Loaded { project, .. } => {
                project.update(cx, |project, _| project.name = new_name)
            }
            _ => {}
        }

        cx.emit(WorkspaceEvent::ProjectsChanged);
        self.save(cx);
    }

    /// Removes a project from the workspace
    ///
    /// If the project was active, set the active project to *None*
    /// # Events
    /// Emits a [`WorkspaceEvent::ProjectsChanged`] if the project is successfully removed from the workspace
    ///
    /// Emits a [`WorkspaceEvent::ActiveProjectChanged`] if the active project is modified
    pub fn remove_project(&mut self, project_path: &PathBuf, cx: &mut Context<Self>) {
        let Some(_) = self.workspace_projects.remove(project_path) else {
            return;
        };

        if self.active_project.as_ref() == Some(project_path) {
            self.active_project = None;
            cx.emit(WorkspaceEvent::ActiveProjectChanged);
        }

        cx.emit(WorkspaceEvent::ProjectsChanged);
        self.save(cx);
    }

    /// Saves the workspace file in a background thread
    fn save(&self, cx: &mut Context<Self>) {
        let config_dir = cx.read_global(|settings: &GlobalSettings, _| settings.config_dir.clone());
        let file_path = config_dir.join(WORKSPACES_FILENAME);

        cx.spawn(async move |this, cx| {
            _ = this.read_with(cx, |this, cx| {
                if let Err(err) = fs::create_dir_all(&*config_dir)
                    .map_err(|e| WorkspaceError::Io(e.to_string()))
                    .and_then(|_| {
                        fs::File::create(&file_path).map_err(|e| WorkspaceError::Io(e.to_string()))
                    })
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

    /// Switch the active project to the one referenced by the given path
    ///
    /// If the project is already does nothing
    ///
    /// In both cases, the project is set as the currently active project
    /// # Events
    /// Emits a [`WorkspaceEvent::ActiveProjectChanged`] if the project is successfully switched
    /// # Returns
    /// If the path is unknown, returns a wrapped [`ProjectError::UnknownProject`]
    ///
    /// If the path references a project that failed to load, returns a [`WorkspaceError::ProjectNotLoaded`]
    pub fn switch_project(&mut self, project_path: PathBuf, cx: &mut Context<Self>) -> Result<()> {
        let Some(project) = self.workspace_projects.get(&project_path) else {
            return Err(ProjectError::UnknownProject(project_path).into());
        };

        if !project.is_loaded() {
            Err(ProjectNotLoaded)
        } else {
            self.active_project = Some(project_path.clone());
            cx.emit(WorkspaceEvent::ActiveProjectChanged);
            self.save(cx);
            Ok(())
        }
    }

    /// Converts the workspace to a serializable [`WorkspaceFile`]
    fn to_file(&self, cx: &App) -> WorkspaceFile {
        WorkspaceFile {
            active_project: self.active_project.clone(),
            projects: self
                .workspace_projects
                .iter()
                .map(|(path, workspace_project)| match &workspace_project.data {
                    WorkspaceProjectData::Loaded { project, .. } => {
                        let project = project.read(cx);
                        FileProjectMetadata {
                            name: workspace_project.name.to_string(),
                            path: path.clone(),
                            active_profile: project.active_profile(),
                        }
                    }
                    WorkspaceProjectData::Error(_, metadata) => metadata.clone(),
                })
                .collect(),
        }
    }

    /// Project event's handler, mostly used to serialize to file project's settings
    fn on_project_event(
        &mut self,
        _project: Entity<Project>,
        event: &ProjectEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            ProjectEvent::ActiveProfileChanged(_) => self.save(cx),
        }
    }
}

impl EventEmitter<WorkspaceEvent> for Workspace {}

/// Container for a registered project in the workspace
pub struct WorkspaceProject {
    name: SharedString,
    data: WorkspaceProjectData,
}

impl WorkspaceProject {
    fn project(
        project: Entity<Project>,
        project_event_sub: Subscription,
        metadata: &FileProjectMetadata,
    ) -> Self {
        Self {
            name: SharedString::new(metadata.name.clone()),
            data: WorkspaceProjectData::Loaded {
                project,
                _sub: project_event_sub,
            },
        }
    }

    fn error(err: ProjectError, metadata: FileProjectMetadata) -> Self {
        Self {
            name: SharedString::new(metadata.name.clone()),
            data: WorkspaceProjectData::Error(err, metadata),
        }
    }
}

enum WorkspaceProjectData {
    /// Stores the error generated when a laoding attempt was made, and the metadata
    Error(ProjectError, FileProjectMetadata),
    Loaded {
        project: Entity<Project>,
        _sub: Subscription,
    },
}

impl WorkspaceProject {
    fn is_loaded(&self) -> bool {
        matches!(self.data, WorkspaceProjectData::Loaded { .. })
    }
}

#[derive(Clone)]
pub struct WorkspaceProjectInfo {
    pub name: SharedString,
    pub path: PathBuf,
    pub loaded: bool,
    pub active: bool,
}
