use crate::workspace::endpoint::WorkspaceEndpoint;
use crate::workspace::error::ProjectError;
use crate::workspace::test::TestsContainer;
use crate::workspace::variable::{Profile, ProjectVariables};
use chrono::{DateTime, Local};
use gpui::{Context, EventEmitter, SharedString, Window};
use gpui_component::WindowExt;
use ki_project::{ProjectFile};
use log::{error, warn};
use std::path::PathBuf;
use uuid::Uuid;

pub const PROJECT_FILE_EXT: &str = "kpr";

/// Result alias for Workspace
pub type Result<T> = std::result::Result<T, ProjectError>;

///// WORKSPACE PROJECT EVENTS /////
pub enum ProjectEvent {
    /// Emitted when the active profile has been modified
    ActiveProfileChanged(Option<Uuid>),
    /// Emitted when the list has been modified (profile added, removed, modified...)
    ProfilesChanged,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Project {
    pub name: SharedString,
    created: DateTime<Local>,
    modified: DateTime<Local>,
    pub variables: ProjectVariables,
    pub endpoints: Vec<WorkspaceEndpoint>,
    pub tests: TestsContainer,
    // unserialized data
    active_profile: Option<Uuid>,
    pub(super) path: PathBuf,
}

impl Project {
    #[inline]
    pub fn active_profile(&self) -> Option<Uuid> {
        self.active_profile.clone()
    }

    /// Profile Management

    /// Adds a profile with the given name
    ///
    /// if profiles were successfully changed, a [`ProjectEvent::ProfilesChanged`] event will be emitted
    ///
    /// # Return
    /// Returns a copy of the updated profiles
    pub fn add_profile(
        &mut self,
        index: Option<usize>,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Profile> {
        let profiles = self.variables.add_profile(index, name);
        self.save(Some(ProjectEvent::ProfilesChanged), window, cx);

        profiles
    }

    /// Duplicates a profile, attributing a new id and copying all fields. Name is appended with *_copy*.
    ///
    /// The copy is placed right after the original
    ///
    /// if profile was successfully removed, a [`ProjectEvent::ProfilesChanged`] event will be emitted.
    /// Additionally, a [`ProjectEvent::ActiveProfileChanged`] event will also be emitted if the profile was active, and the active profile set to *None*
    ///
    /// # Return
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound, a copy of the updated profiles else
    pub fn delete_profile(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        let (profiles, removed_profile) = self.variables.delete_profile(index)?;
        self.save(Some(ProjectEvent::ProfilesChanged), window, cx);

        // check if deleted profile was the active one
        if Some(removed_profile.id) == self.active_profile {
            self.switch_profile(None, cx);
        }

        Ok(profiles)
    }

    /// Deletes a profile a returns a copy of the new list and the removed profile, for further processing
    ///
    /// if profiles were successfully changed, a [`ProjectEvent::ProfilesChanged`] event will be emitted
    ///
    /// # Return
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound, a copy of the updated profiles else
    pub fn duplicate_profile(
        &mut self,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        let result = self.variables.duplicate_profile(row);
        if result.is_ok() {
            self.save(Some(ProjectEvent::ProfilesChanged), window, cx);
        }
        result
    }

    /// Moves a profile from one position to another
    /// # Events
    /// Emits a [`ProjectEvent::ProfilesChanged`] if profile was successfully moved
    ///
    /// # Return
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound, a copy of the updated profiles else
    pub fn move_profile(
        &mut self,
        row_from: usize,
        row_to: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        let result = self.variables.move_profile(row_from, row_to);
        if result.is_ok() {
            self.save(Some(ProjectEvent::ProfilesChanged), window, cx);
        }
        result
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
                if self.variables.profiles.iter().any(|p| p.id == profile_id) {
                    self.active_profile = Some(profile_id);
                    cx.emit(ProjectEvent::ActiveProfileChanged(self.active_profile));
                }
            }
        }
    }

    /// Merges a profile attributes
    /// # Events
    /// if profile was successfully changed, a [`ProjectEvent::ProfilesChanged`] event will be emitted
    /// # Return
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound, a copy of the updated profiles else
    pub fn update_profile(
        &mut self,
        updated_profile: &Profile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        let result = self.variables.update_profile(updated_profile);
        if result.is_ok() {
            self.save(Some(ProjectEvent::ProfilesChanged), window, cx);
        }
        result
    }

    pub(super) fn load(path: &PathBuf, active_profile: Option<Uuid>) -> Result<Self> {
        if !path.exists() || path.extension() != Some(PROJECT_FILE_EXT.as_ref()) {
            error!("Invalid project at: {}", path.to_string_lossy());
            return Err(ProjectError::BadLocation(path.clone()));
        }

        let file_project = ProjectFile::load(&path).map_err(|err| {
            error!("Failed to load project at: {}. Error: {:?}", path.to_string_lossy(), err);
            ProjectError::from(err)
        })?;
        let variables = ProjectVariables::from_file(&file_project.variables);
        let endpoints = WorkspaceEndpoint::from_file(&file_project.endpoints);
        let tests = TestsContainer::from_file(&file_project.tests);
        let active_profile = match active_profile {
            None => None,
            Some(id) => variables.profiles.iter().find(|p| p.id == id).map(|p| p.id),
        };

        Ok(Self {
            name: SharedString::new(file_project.name),
            created: file_project.created,
            modified: file_project.modified,
            variables,
            endpoints,
            tests,
            active_profile,
            path: path.clone(),
        })
    }

    pub(super) fn new(
        path: PathBuf,
        name: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self {
            path,
            name,
            created: Default::default(),
            modified: Default::default(),
            variables: Default::default(),
            endpoints: vec![],
            tests: Default::default(),
            active_profile: None,
        };
        this.save(None, window, cx);
        this
    }

    /// save the project to file and emit the passed event
    pub fn save(
        &mut self,
        event: Option<ProjectEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            if let Some(this) = this.upgrade() {
                if let Err(err) = this.update(cx, |this, _| {
                    let project = this.to_file();
                    project
                        .save(&this.path)
                        .map_err(|e| ProjectError::from(e))?;
                    // Update the 'modified' attribute if save was successful
                    this.modified = project.modified;
                    Ok::<(), ProjectError>(())
                }) {
                    _ = cx.update(|window, cx| window.push_notification(err, cx));
                }
            }
        })
        .detach();
        if let Some(event) = event {
            cx.emit(event);
        }
    }

    fn to_file(&self) -> ProjectFile {
        let data = self;
        let variables = data.variables.to_file();
        let endpoints = data.endpoints.iter().map(|e| e.to_file()).collect();
        let tests = data.tests.to_file();
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
}

impl EventEmitter<ProjectEvent> for Project {}
