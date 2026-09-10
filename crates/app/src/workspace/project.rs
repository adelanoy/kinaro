use crate::workspace::endpoint::WorkspaceEndpoint;
use crate::workspace::error::ProjectError;
use crate::workspace::test::{TestsContainer, TestsContainerEvent};
use crate::workspace::variable::{ProjectVariables, ProjectVariablesEvent};
use crate::workspace::{FileProjectMetadata, Workspace};
use chrono::{DateTime, Local};
use gpui_kit::{AppContext, Context, Entity, EventEmitter, SharedString, Subscription};
use ki_project::ProjectFile;
use log::error;
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

pub const PROJECT_FILE_EXT: &str = "kpr";

/// Result alias for Workspace
pub type Result<T> = std::result::Result<T, ProjectError>;

///// WORKSPACE PROJECT EVENTS /////
pub enum ProjectEvent {
  /// Emitted when the active profile has been modified
  ActiveProfileChanged(Option<Uuid>),
  TreeNodesChanged,
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
  pub(super) path: PathBuf,
  _variables_event_sub: Subscription,
  _tests_event_sub: Subscription,
  active_profile: Option<Uuid>,
  opened_tree_nodes: HashSet<Uuid>,
}

impl Project {
  #[inline]
  pub fn active_profile(&self) -> Option<Uuid> {
    self.active_profile
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

  #[inline]
  pub fn opened_tree_nodes(&self) -> &HashSet<Uuid> {
    &self.opened_tree_nodes
  }

  pub fn expand_tree_node(&mut self, id: Uuid, cx: &mut Context<Self>) {
    self.opened_tree_nodes.insert(id);
    cx.emit(ProjectEvent::TreeNodesChanged);
  }

  pub fn collapse_tree_node(&mut self, id: &Uuid, cx: &mut Context<Self>) {
    self.opened_tree_nodes.remove(id);
    cx.emit(ProjectEvent::TreeNodesChanged);
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
      let tests = cx.new(|_| TestsContainer::from_file(file_project.tests));
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
        opened_tree_nodes: metadata.opened_tree_nodes.clone(),
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
      opened_tree_nodes: HashSet::new(),
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
      cx.emit(ProjectEvent::ActiveProfileChanged(self.active_profile));
    }

    self.save(cx);
  }

  fn on_tests_event(
    &mut self,
    _tests: Entity<TestsContainer>,
    e: &TestsContainerEvent,
    cx: &mut Context<Self>,
  ) {
    // Check if the currently active profile has been deleted
    if let TestsContainerEvent::Removed(id) = e {
      self.opened_tree_nodes.remove(id);
    }
    cx.emit(ProjectEvent::TreeNodesChanged);
    self.save(cx);
  }
}

impl EventEmitter<ProjectEvent> for Project {}
