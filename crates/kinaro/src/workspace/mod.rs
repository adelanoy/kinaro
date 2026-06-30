use crate::workspace::error::{ProjectError, WorkspaceError};
pub(crate) use crate::workspace::project::{
    WorkspaceProject, WorkspaceProjectDataStatus, WorkspaceProjectEvent,
};
use gpui::{Action, App, Entity, EventEmitter, SharedString, actions};
use gpui::{AppContext, Context};
use ki_project::ProjectFile;
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

pub const WORKSPACES_FILENAME: &str = "workspace.json";
pub const PROJECT_FILE_EXT: &str = "kpr";

/// Result alias for Workspace
pub type Result<T> = std::result::Result<T, WorkspaceError>;

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
    ActiveProjectChanged,
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
            .map(|project| {
                let project_entity = cx.new(|_| project.load());

                let project_id = project_entity.read(cx).id;
                // If active project and loading failed, reset active project
                if Some(project_id) == active_project_id && !project_entity.read(cx).is_loaded() {
                    active_project_id = None;
                }
                (project_id, project_entity)
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
            cx.emit(WorkspaceEvent::ActiveProjectChanged);
        }
        result
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
        let project_file =
            ProjectFile::load(&path).map_err(|err| WorkspaceError::Project(err.into()))?;
        let project = cx.new(|_| WorkspaceProject::read_project(path, project_file));
        let project_id = project.read(cx).id;
        self.active_project_id = Some(project_id);
        self.projects.insert(project_id, project);

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
