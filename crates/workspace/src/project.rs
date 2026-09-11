use crate::endpoint::WorkspaceEndpoint;
use crate::error::ProjectError;
use crate::test::{TestsContainer, TestsContainerEvent};
use crate::variable::{ProjectVariables, ProjectVariablesEvent};
use crate::{FileProjectMetadata, Workspace};
use chrono::{DateTime, Local};
use gpui_kit::{App, AppContext, Context, Entity, EventEmitter, SharedString, Subscription};
use ki_project::ProjectFile;
use log::error;
use std::path::PathBuf;
use uuid::Uuid;

pub const PROJECT_FILE_EXT: &str = "kpr";

/// Result alias for Workspace
pub type Result<T> = std::result::Result<T, ProjectError>;

///// WORKSPACE PROJECT EVENTS /////
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectEvent {
  /// Emitted when the active profile has been modified
  ActiveProfile(Option<Uuid>),
  /// Emitted when the test tree content has changed
  TestTree,
  /// Emitted when the test tree nodes have changed
  TestTreeNodes,
}

#[derive(Debug)]
pub struct Project {
  pub name: SharedString,
  created: DateTime<Local>,
  modified: DateTime<Local>,
  pub variables: Entity<ProjectVariables>,
  pub endpoints: Vec<WorkspaceEndpoint>,
  pub tests: Entity<TestsContainer>,
  // unserialized data
  path: PathBuf,
  _variables_event_sub: Subscription,
  _tests_event_sub: Subscription,
  active_profile: Option<Uuid>,
}

impl Project {
  #[inline]
  pub fn active_profile(&self) -> Option<Uuid> {
    self.active_profile
  }

  /// Change the currently active profile
  /// # Events
  /// Emits a [`ProjectEvent::ActiveProfile`] if active profile was successfully changed
  pub fn switch_profile(&mut self, profile_id: Option<Uuid>, cx: &mut Context<Self>) {
    if profile_id == self.active_profile {
      return;
    }
    match profile_id {
      None => {
        self.active_profile = None;
        cx.emit(ProjectEvent::TestTreeNodes);
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
          cx.emit(ProjectEvent::ActiveProfile(self.active_profile));
        }
      }
    }
  }

  pub(super) fn load(
    metadata: &FileProjectMetadata,
    cx: &mut Context<Workspace>,
  ) -> Result<Entity<Self>> {
    let path = &metadata.path;
    if !path.exists() || path.extension() != Some(PROJECT_FILE_EXT.as_ref()) {
      error!("Invalid project at: {}", path.to_string_lossy());
      return Err(ProjectError::BadLocation(path.to_owned()));
    }

    let file_project = ProjectFile::load(path).map_err(|err| {
      let error = ProjectError::from(err);
      error!(
                "Failed to load project at: {}. Error: {:?}",
                path.to_string_lossy(),
                error
            );
      error
    })?;
    let this = cx.new(|cx| {
      let variables = cx.new(|_| ProjectVariables::from_file(file_project.variables));
      let _variables_event_sub = cx.subscribe(&variables, Self::on_profiles_variables_event);
      let endpoints = WorkspaceEndpoint::from_file(&file_project.endpoints);
      let tests = cx.new(|_| TestsContainer::from_file(file_project.tests, metadata));
      let _tests_event_sub = cx.subscribe(&tests, Self::on_tests_event);
      let active_profile = metadata
        .active_profile
        .filter(|id| variables.read(cx).profiles.iter().any(|p| p.id == *id));
      Self {
        name: SharedString::new(file_project.name),
        created: file_project.created,
        modified: file_project.modified,
        variables,
        endpoints,
        tests,
        _variables_event_sub,
        _tests_event_sub,
        active_profile,
        path: path.to_owned(),
      }
    });

    Ok(this)
  }

  pub(super) fn new(path: PathBuf, name: SharedString, cx: &mut Context<Self>) -> Self {
    let variables = cx.new(|_| Default::default());
    let _variables_event_sub = cx.subscribe(&variables, Self::on_profiles_variables_event);
    let tests = cx.new(|_| Default::default());
    let _tests_event_sub = cx.subscribe(&tests, Self::on_tests_event);
    let mut this = Self {
      path,
      name,
      created: Default::default(),
      modified: Default::default(),
      variables,
      endpoints: vec![],
      _variables_event_sub,
      _tests_event_sub,
      tests,
      active_profile: None,
    };
    this.save(cx);
    this
  }

  /// save the project to file and emit the passed event
  pub fn save(&mut self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
      if let Some(this) = this.upgrade()
        && let Err(err) = this.update(cx, |this, cx| {
        let project = this.to_file(cx);
        project.save(&this.path).map_err(ProjectError::from)?;
        // Update the 'modified' attribute if save was successful
        this.modified = project.modified;
        Ok::<(), ProjectError>(())
      })
      {
        error!("Failed to save project file: {}", err);
      }
    })
      .detach();
  }

  pub(super) fn metadata(&self, cx: &App) -> FileProjectMetadata {
    FileProjectMetadata {
      path: self.path.clone(),
      active_profile: self.active_profile,
      opened_tree_nodes: self.tests.read(cx).opened_tree_nodes().clone(),
    }
  }

  fn to_file(&self, cx: &Context<Self>) -> ProjectFile {
    let variables = self.variables.read(cx).to_file();
    let endpoints = self.endpoints.iter().map(|e| e.to_file()).collect();
    let tests = self.tests.read(cx).to_file();
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
    // Check if the currently active profile has been deleted
    if let ProjectVariablesEvent::ProfilesChanged = e
      && let Some(active_profile) = self.active_profile
      && !project_vars
      .read(cx)
      .profiles
      .iter()
      .any(|p| p.id == active_profile)
    {
      self.active_profile = None;
      cx.emit(ProjectEvent::ActiveProfile(self.active_profile));
    }

    self.save(cx);
  }

  fn on_tests_event(
    &mut self,
    _tests: Entity<TestsContainer>,
    event: &TestsContainerEvent,
    cx: &mut Context<Self>,
  ) {
    match event {
      TestsContainerEvent::TestsModified => {
        self.save(cx);
        cx.emit(ProjectEvent::TestTree);
      }
      TestsContainerEvent::TreeNodesChanged => cx.emit(ProjectEvent::TestTreeNodes),
    }
  }
}

impl EventEmitter<ProjectEvent> for Project {}
