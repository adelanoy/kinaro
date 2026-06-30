use crate::workspace::endpoint::WorkspaceEndpoint;
use crate::workspace::error::ProjectError;
use crate::workspace::test::TestsContainer;
use crate::workspace::variable::{Profile, ProjectVariables};
use chrono::{DateTime, Local};
use gpui::{Context, EventEmitter, SharedString, Window};
use gpui_component::WindowExt;
use ki_project::{FileProject, ProjectFileError};
use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Result alias for Workspace
pub type Result<T> = std::result::Result<T, ProjectError>;

///// WORKSPACE PROJECT EVENTS /////
pub enum ProjectEvent {
    /// Emitted when the active profile has been modified
    ActiveProfileChanged(Option<Uuid>),
    /// Emitted when the list has been modified (profile added, removed, modified...)
    ProfilesChanged,
}

impl EventEmitter<ProjectEvent> for Project {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub path: PathBuf,
    pub id: Uuid,
    pub name: SharedString,
    pub version: u16,
    pub created: DateTime<Local>,
    pub modified: DateTime<Local>,
    pub(super) active_profile: Option<Uuid>,
    #[serde(skip)]
    data_status: ProjectDataStatus,
}

impl Clone for Project {
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

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Project {
    #[inline]
    pub fn active_profile(&self) -> Option<Uuid> {
        self.active_profile.clone()
    }

    #[inline]
    pub fn is_loaded(&self) -> bool {
        matches!(self.data_status, ProjectDataStatus::Loaded(_))
    }

    #[inline]
    pub fn get_status(&self) -> &ProjectDataStatus {
        &self.data_status
    }

    /// Returns a reference to the data held by the project.
    /// Care should be taken to check the project status as this method will panic if the project is not loaded
    pub fn data(&self) -> &ProjectData {
        match &self.data_status {
            ProjectDataStatus::Loaded(data) => data,
            _ => panic!("Tried to read an unloaded project"),
        }
    }

    /// Returns a reference to the data held by the project.
    /// Care should be taken to check the project status as this method will panic if the project is not loaded
    pub fn data_mut(&mut self) -> &mut ProjectData {
        match &mut self.data_status {
            ProjectDataStatus::Loaded(data) => data,
            _ => panic!("Tried to read an unloaded project"),
        }
    }

    /// Profile Management

    /// Adds a profile with the given name
    ///
    /// if profiles were successfully changed, a [`ProjectEvent::ProfilesChanged`] event will be emitted
    ///
    /// # Result
    /// Returns a [`ProjectError::Invalid`] if this was called on an unloaded project
    pub fn add_profile(
        &mut self,
        index: Option<usize>,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        if !self.is_loaded() {
            return Err(ProjectError::Invalid);
        }
        let profiles = self.data_mut().variables.add_profile(index, name);
        self.save(ProjectEvent::ProfilesChanged, window, cx);

        Ok(profiles)
    }

    /// Duplicates a profile, attributing a new id and copying all fields. Name is appended with *_copy*.
    ///
    /// The copy is placed right after the original
    ///
    /// if profile was successfully removed, a [`ProjectEvent::ProfilesChanged`] event will be emitted.
    /// Additionally, a [`ProjectEvent::ActiveProfileChanged`] event will also be emitted if the profile was active, and the active profile set to *None*
    ///
    /// # Result
    /// Returns a [`ProjectError::Invalid`] if this was called on an unloaded project
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
    pub fn delete_profile(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        if !self.is_loaded() {
            return Err(ProjectError::Invalid);
        }
        let (profiles, removed_profile) = self.data_mut().variables.delete_profile(index)?;
        self.save(ProjectEvent::ProfilesChanged, window, cx);

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
    /// # Result
    /// Returns a [`ProjectError::Invalid`] if this was called on an unloaded project
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
    pub fn duplicate_profile(
        &mut self,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        if !self.is_loaded() {
            return Err(ProjectError::Invalid);
        }
        let result = self.data_mut().variables.duplicate_profile(row);
        if result.is_ok() {
            self.save(ProjectEvent::ProfilesChanged, window, cx);
        }
        result
    }

    /// Moves a profile from one position to another
    ///
    /// if profiles were successfully changed, a [`ProjectEvent::ProfilesChanged`] event will be emitted
    ///
    /// # Result
    /// Returns a [`ProjectError::Invalid`] if this was called on an unloaded project
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
    pub fn move_profile(
        &mut self,
        row_from: usize,
        row_to: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        if !self.is_loaded() {
            return Err(ProjectError::Invalid);
        }
        let result = self.data_mut().variables.move_profile(row_from, row_to);
        if result.is_ok() {
            self.save(ProjectEvent::ProfilesChanged, window, cx);
        }
        result
    }

    pub fn switch_profile(&mut self, profile_id: Option<Uuid>, cx: &mut Context<Self>) {
        // TODO: Workspace should listen to active profile change to persist it
        if profile_id == self.active_profile || !self.is_loaded() {
            return;
        }
        match profile_id {
            None => {
                self.active_profile = None;
                cx.emit(ProjectEvent::ActiveProfileChanged(None));
            }
            Some(profile_id) => {
                if self
                    .data()
                    .variables
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

    /// Merges a profile attributes
    ///
    /// if profile was successfully changed, a [`ProjectEvent::ProfilesChanged`] event will be emitted
    ///
    /// # Result
    /// Returns a [`ProjectError::Invalid`] if this was called on an unloaded project
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
    pub fn update_profile(
        &mut self,
        updated_profile: &Profile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Profile>> {
        if !self.is_loaded() {
            return Err(ProjectError::Invalid);
        }
        let result = self.data_mut().variables.update_profile(updated_profile);
        if result.is_ok() {
            self.save(ProjectEvent::ProfilesChanged, window, cx);
        }
        result
    }

    pub(super) fn load(mut self) -> Self {
        if !self.path.exists() {
            warn!("Could not find project at: {}", self.path.to_string_lossy());
            self.data_status = ProjectDataStatus::Moved;
            return self;
        }

        match FileProject::load(&self.path) {
            Ok(project_file) => {
                if project_file.id != self.id {
                    warn!(
                        "Project id mismatch for project at: {}. Workspace id: {}, file id: {}",
                        self.path.to_string_lossy(),
                        self.id,
                        project_file.id
                    );
                    self.data_status = ProjectDataStatus::ExternallyModified;
                }
                let project = ProjectData::from_file(&project_file);
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
                self.data_status = ProjectDataStatus::Loaded(project)
            }
            Err(err) => {
                error!(
                    "Error loading project at: {}. Error: {}",
                    self.path.to_string_lossy(),
                    err
                );
                self.data_status = ProjectDataStatus::LoadError(err);
            }
        };

        self
    }

    pub(super) fn read_project(path: PathBuf, project_file: FileProject) -> Self {
        let data = ProjectData::from_file(&project_file);
        Self {
            path,
            id: project_file.id,
            name: SharedString::new(project_file.name),
            version: 1,
            created: project_file.created,
            modified: project_file.modified,
            active_profile: None,
            data_status: ProjectDataStatus::Loaded(data),
        }
    }

    pub(super) fn new(path: PathBuf, name: String) -> Self {
        Self {
            path,
            id: Uuid::new_v4(),
            name: SharedString::new(name),
            version: 1,
            created: Default::default(),
            modified: Default::default(),
            active_profile: None,
            data_status: ProjectDataStatus::Loaded(ProjectData::new()),
        }
    }

    /// save the project to file and emit the passed event
    pub fn save(
        &mut self,
        event: ProjectEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_loaded() {
            return;
        }
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
        cx.emit(event);
    }

    fn to_file(&self) -> FileProject {
        let data = self.data();
        let variables = data.variables.to_file();
        let endpoints = data.endpoints.iter().map(|e| e.to_file()).collect();
        let tests = data.tests.to_file();
        let modified = Local::now();

        FileProject {
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
pub enum ProjectDataStatus {
    Unloaded,
    Loaded(ProjectData),
    Moved,
    ExternallyModified,
    LoadError(ProjectFileError),
}

impl Default for ProjectDataStatus {
    fn default() -> Self {
        ProjectDataStatus::Unloaded
    }
}

#[derive(Clone, Debug)]
pub struct ProjectData {
    pub variables: ProjectVariables,
    pub endpoints: Vec<WorkspaceEndpoint>,
    pub tests: TestsContainer,
}

impl ProjectData {
    fn new() -> Self {
        Self {
            variables: Default::default(),
            endpoints: vec![],
            tests: Default::default(),
        }
    }

    fn from_file(project: &FileProject) -> ProjectData {
        let variables = ProjectVariables::from_file(&project.variables);
        let endpoints = WorkspaceEndpoint::from_file(&project.endpoints);
        let tests = TestsContainer::from_file(&project.tests);

        Self {
            variables,
            endpoints,
            tests,
        }
    }
}
