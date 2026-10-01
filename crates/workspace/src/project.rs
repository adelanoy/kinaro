use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestsContainer, TestsContainerEvent};
use crate::variable::{ProjectVariables, ProjectVariablesEvent};
use crate::{FileProjectMetadata, Workspace};
use chrono::{DateTime, Local};
use gpui_kit::{App, AppContext, Context, Entity, EventEmitter, SharedString, Subscription, Task};
use ki_project::ProjectFile;
use log::{debug, error};
use std::path::PathBuf;
use uuid::Uuid;

pub const PROJECT_FILE_EXT: &str = "kpr";

///// WORKSPACE PROJECT EVENTS /////
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectEvent {
  /// Emitted when the active profile has been modified
  ActiveProfile(Option<Uuid>),
  /// Emitted when the test tree content has changed (the project file has been saved)
  TestTreeData,
  /// Emitted when the project's configuration (expanded tree nodes, editor tabs...) has changed
  /// and the workspace should save itself
  ProjectConfigChanged,
}

#[derive(Debug)]
pub struct Project {
  pub name: SharedString,
  created: DateTime<Local>,
  modified: DateTime<Local>,
  pub variables: Entity<ProjectVariables>,
  pub tests: Entity<TestsContainer>,
  // unserialized data
  path: PathBuf,
  _variables_event_sub: Subscription,
  _tests_event_sub: Subscription,
  active_profile: Option<Uuid>,
  /// Save throttle handle
  save_task_queued: Option<Task<()>>,
}

impl Project {
  #[inline]
  pub fn active_profile(&self) -> Option<Uuid> {
    self.active_profile
  }

  pub(super) fn load(metadata: &FileProjectMetadata, cx: &mut Context<Workspace>) -> ProjectResult<Entity<Self>> {
    let path = &metadata.path;
    if !path.exists() || path.extension() != Some(PROJECT_FILE_EXT.as_ref()) {
      error!("Invalid project at: {}", path.to_string_lossy());
      return Err(ProjectError::BadLocation(path.to_owned()));
    }

    let file_project = ProjectFile::load(path).map_err(|err| {
      let error = ProjectError::from(err);
      error!("Failed to load project at: {}. Error: {:?}", path.to_string_lossy(), error);
      error
    })?;
    let this = cx.new(|cx| {
      Self::register_quit_callback(path, cx);

      let variables = cx.new(|_| ProjectVariables::from_file(file_project.variables));
      let _variables_event_sub = cx.subscribe(&variables, Self::on_profiles_variables_event);
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
        tests,
        _variables_event_sub,
        _tests_event_sub,
        active_profile,
        path: path.to_owned(),
        save_task_queued: None,
      }
    });

    Ok(this)
  }

  pub(super) fn new(path: PathBuf, name: SharedString, cx: &mut Context<Self>) -> Self {
    Self::register_quit_callback(&path, cx);

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
      _variables_event_sub,
      _tests_event_sub,
      tests,
      active_profile: None,
      save_task_queued: None,
    };
    this.save(cx);

    this
  }

  pub(super) fn metadata(&self, cx: &App) -> FileProjectMetadata {
    let (opened_tree_nodes, opened_editor_nodes, active_editor_index) = self.tests.read_with(cx, |tests, _| {
      (
        tests.opened_tree_nodes().clone(),
        tests.opened_editor_nodes().clone(),
        tests.active_editor_tab_index(),
      )
    });
    FileProjectMetadata {
      path: self.path.clone(),
      active_profile: self.active_profile,
      opened_tree_nodes,
      editor_tabs: opened_editor_nodes,
      active_editor_tab_index: active_editor_index,
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
      && !project_vars.read(cx).profiles.iter().any(|p| p.id == active_profile)
    {
      self.active_profile = None;
      cx.emit(ProjectEvent::ActiveProfile(self.active_profile));
    }

    self.save(cx);
  }

  fn on_tests_event(&mut self, _tests: Entity<TestsContainer>, event: &TestsContainerEvent, cx: &mut Context<Self>) {
    match event {
      // Trigger project save
      TestsContainerEvent::TestsModified
      | TestsContainerEvent::TestAdded(_)
      | TestsContainerEvent::TestRenamed(_, _, _)
      | TestsContainerEvent::TestMoved(_, _)
      | TestsContainerEvent::TestRemoved(_) => {
        self.save(cx);
        cx.emit(ProjectEvent::TestTreeData)
      }
      // Trigger workspace save
      TestsContainerEvent::ConfigChanged
      | TestsContainerEvent::TestOpenInEditor(_)
      | TestsContainerEvent::TestClosedInEditor(_)
      | TestsContainerEvent::EditorActiveTabIndex(_) => cx.emit(ProjectEvent::ProjectConfigChanged),
      // Internal messages, ignore
      TestsContainerEvent::RequestOpenInEditor(_) => {}
    }
  }

  /// Register a callback on app quitting to force save the project file
  fn register_quit_callback(path: &PathBuf, cx: &mut Context<Self>) {
    cx.on_app_quit({
      let path = path.to_owned();
      move |this, cx| {
        this.save_task_queued = None;
        let bytes = this.to_file(cx);
        cx.background_executor().spawn({
          debug!("Save project file on quit to {}", path.to_string_lossy());
          let path = path.clone();
          async move {
            _ = bytes.save(&path);
          }
        })
      }
    })
    .detach();
  }

  fn to_file(&self, cx: &App) -> ProjectFile {
    let variables = self.variables.read(cx).to_file();
    let tests = self.tests.read(cx).to_file();
    let modified = Local::now();

    ProjectFile {
      name: self.name.to_string(),
      version: 1,
      created: self.created,
      modified,
      variables,
      tests,
    }
  }

  /// Saves the project to its file, asynchronously. Failures are logged.
  pub fn save(&mut self, cx: &mut Context<Self>) {
    if self.save_task_queued.is_some() {
      return;
    }
    let path = self.path.clone();
    self.save_task_queued = Some(cx.spawn(async move |this, cx| {
      let file = this
        .read_with(cx, |this, cx| this.to_file(cx))
        .map_err(|e| ProjectError::Io(e.to_string()));
      let result = match file {
        Ok(file) => cx
          .background_executor()
          .spawn({
            let path = path.clone();
            async move { file.save(&path) }
          })
          .await
          .map_err(ProjectError::from),
        Err(err) => Err(err),
      };
      match result {
        Ok(date_time_modified) => {
          _ = this.update(cx, |this, _| this.modified = date_time_modified);
          debug!("Saved project file to {}", path.to_string_lossy())
        }
        Err(err) => error!("Error while saving project file: {:?}", err),
      }

      _ = this.update(cx, |this, _| {
        this.save_task_queued.take();
      });
    }));
  }

  /// Change the currently active profile. A no-op when `profile_id` is already active or doesn't
  /// address an existing profile.
  ///
  /// # Events
  /// Emits a [`ProjectEvent::ActiveProfile`] if active profile was successfully changed
  pub fn switch_profile(&mut self, profile_id: Option<Uuid>, cx: &mut Context<Self>) {
    if profile_id == self.active_profile {
      return;
    }
    match profile_id {
      None => {
        self.active_profile = None;
        cx.emit(ProjectEvent::ActiveProfile(None));
      }
      Some(profile_id) => {
        if self.variables.read(cx).profiles.iter().any(|p| p.id == profile_id) {
          self.active_profile = Some(profile_id);
          cx.emit(ProjectEvent::ActiveProfile(self.active_profile));
        }
      }
    }
  }
}

impl EventEmitter<ProjectEvent> for Project {}
