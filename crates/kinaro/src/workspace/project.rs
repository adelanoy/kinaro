use crate::workspace::endpoint::WorkspaceEndpoint;
use crate::workspace::error::ProjectError;
use crate::workspace::test::TestsContainer;
use crate::workspace::variable::{ProjectVariables, ProjectVariablesEvent};
use chrono::{DateTime, Local};
use gpui::{AppContext, Context, Entity, EventEmitter, SharedString, Subscription};
use ki_project::ProjectFile;
use log::error;
use std::path::PathBuf;
use uuid::Uuid;
use crate::workspace::Workspace;

pub const PROJECT_FILE_EXT: &str = "kpr";

/// Result alias for Workspace
pub type Result<T> = std::result::Result<T, ProjectError>;

///// WORKSPACE PROJECT EVENTS /////
pub enum ProjectEvent {
    /// Emitted when the active profile has been modified
    ActiveProfileChanged(Option<Uuid>),
}

#[derive(Debug)]
pub struct Project {
    pub name: SharedString,
    created: DateTime<Local>,
    modified: DateTime<Local>,
    pub variables: Entity<ProjectVariables>,
    pub endpoints: Vec<WorkspaceEndpoint>,
    pub tests: TestsContainer,
    // unserialized data
    _variables_event_sub: Subscription,
    active_profile: Option<Uuid>,
    pub(super) path: PathBuf,
}

impl Project {
    #[inline]
    pub fn active_profile(&self) -> Option<Uuid> {
        self.active_profile.clone()
    }


    /// Change the currently active profile
    /// # Events
    /// Emits a [`ProjectEvent::ActiveProfileChanged`] if active profile was successfully changed
    pub fn switch_profile(&mut self, profile_id: Option<Uuid>, cx: &mut Context<Self>) {
        if profile_id == self.active_profile {
            return;
        }
        match profile_id {
            None => {
                self.active_profile = None;
                cx.emit(ProjectEvent::ActiveProfileChanged(None));
            }
            Some(profile_id) => {
                if self
                    .variables
                    .read(cx)
                    .profiles
                    .iter()
                    .any(|p| p.id == profile_id)
                {
                    self.active_profile = Some(profile_id);
                    cx.emit(ProjectEvent::ActiveProfileChanged(self.active_profile));
                }
            }
        }
    }

    pub(super) fn load(
        path: &PathBuf,
        active_profile: Option<Uuid>,
        cx: &mut Context<Workspace>,
    ) -> Result<Entity<Self>> {
        if !path.exists() || path.extension() != Some(PROJECT_FILE_EXT.as_ref()) {
            error!("Invalid project at: {}", path.to_string_lossy());
            return Err(ProjectError::BadLocation(path.clone()));
        }

        let file_project = ProjectFile::load(&path).map_err(|err| {
            let error = ProjectError::from(err);
            error!(
                "Failed to load project at: {}. Error: {:?}",
                path.to_string_lossy(),
                error
            );
            error
        })?;
        let this = cx.new(|cx| {
            let variables = cx.new(|_| ProjectVariables::from_file(&file_project.variables));
            let _variables_event_sub = cx.subscribe(&variables, Self::on_profiles_variables_event);
            let endpoints = WorkspaceEndpoint::from_file(&file_project.endpoints);
            let tests = TestsContainer::from_file(&file_project.tests);
            let active_profile = match active_profile {
                None => None,
                Some(id) => variables
                    .read(cx)
                    .profiles
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.id),
            };
            Self {
                name: SharedString::new(file_project.name),
                created: file_project.created,
                modified: file_project.modified,
                variables,
                endpoints,
                tests,
                _variables_event_sub,
                active_profile,
                path: path.clone(),
            }
        });

        Ok(this)
    }

    pub(super) fn new(path: PathBuf, name: SharedString, cx: &mut Context<Self>) -> Self {
        let variables = cx.new(|_| Default::default());
        let _variables_event_sub = cx.subscribe(&variables, Self::on_profiles_variables_event);
        let mut this = Self {
            path,
            name,
            created: Default::default(),
            modified: Default::default(),
            variables,
            endpoints: vec![],
            _variables_event_sub,
            tests: Default::default(),
            active_profile: None,
        };
        this.save(cx);
        this
    }

    /// save the project to file and emit the passed event
    pub fn save(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            if let Some(this) = this.upgrade() {
                if let Err(err) = this.update(cx, |this, cx| {
                    let project = this.to_file(cx);
                    project
                        .save(&this.path)
                        .map_err(|e| ProjectError::from(e))?;
                    // Update the 'modified' attribute if save was successful
                    this.modified = project.modified;
                    Ok::<(), ProjectError>(())
                }) {
                    error!("Failed to save project file: {}", err);
                }
            }
        })
        .detach();
    }

    fn to_file(&self, cx: &Context<Self>) -> ProjectFile {
        let variables = self.variables.read(cx).to_file();
        let endpoints = self.endpoints.iter().map(|e| e.to_file()).collect();
        let tests = self.tests.to_file();
        let modified = Local::now();

        ProjectFile {
            name: self.name.to_string(),
            version: 1,
            created: self.created,
            modified,
            variables,
            endpoints,
            tests,
        }
    }

    fn on_profiles_variables_event(
        &mut self,
        project_vars: Entity<ProjectVariables>,
        e: &ProjectVariablesEvent,
        cx: &mut Context<Self>,
    ) {
        match e {
            // Check if the currently active profile has been deleted
            ProjectVariablesEvent::ProfilesChanged => match self.active_profile {
                Some(active_profile) => {
                    if !project_vars
                        .read(cx)
                        .profiles
                        .iter()
                        .any(|p| p.id == active_profile)
                    {
                        self.active_profile = None;
                        cx.emit(ProjectEvent::ActiveProfileChanged(self.active_profile));
                    }
                }
                None => {}
            }
            _ => {},
        }

        self.save(cx);
    }
}

impl EventEmitter<ProjectEvent> for Project {}
