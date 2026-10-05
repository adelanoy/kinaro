use crate::error::{ProjectError, ProjectResult};
use crate::test::{TestsContainer, TestsContainerEvent};
use crate::variable::{ProjectVariables, ProjectVariablesEvent};
use crate::{FileProjectMetadata, Workspace};
use chrono::{DateTime, Local};
use gpui_kit::{App, AppContext, Context, Entity, EventEmitter, SharedString, Subscription, Task};
use ki_project::ProjectFile;
use log::{debug, error};
use std::path::PathBuf;
use std::time::Duration;

pub const PROJECT_FILE_EXT: &str = "kpr";

///// WORKSPACE PROJECT EVENTS /////
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectEvent {
  /// Emitted when the project content has changed (the project file has been saved)
  ProjectDataChanged,
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
  /// Save throttle handle
  save_task_queued: Option<Task<()>>,
  /// Whether a save was requested while the queued save was running, which may have already serialized
  save_requested: bool,
}

impl Project {
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

      let variables = cx.new(|_| ProjectVariables::from_file(file_project.variables, metadata));
      let _variables_event_sub = cx.subscribe(&variables, Self::on_profiles_variables_event);
      let tests = cx.new(|_| TestsContainer::from_file(file_project.tests, metadata));
      let _tests_event_sub = cx.subscribe(&tests, Self::on_tests_event);
      Self {
        name: SharedString::new(file_project.name),
        created: file_project.created,
        modified: file_project.modified,
        variables,
        tests,
        _variables_event_sub,
        _tests_event_sub,
        path: path.to_owned(),
        save_task_queued: None,
        save_requested: false,
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
      save_task_queued: None,
      save_requested: false,
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
    let active_profile = self.variables.read(cx).active_profile();
    FileProjectMetadata {
      path: self.path.clone(),
      active_profile,
      opened_tree_nodes,
      editor_tabs: opened_editor_nodes,
      active_editor_tab_index: active_editor_index,
    }
  }

  fn on_profiles_variables_event(
    &mut self,
    _project_vars: Entity<ProjectVariables>,
    event: &ProjectVariablesEvent,
    cx: &mut Context<Self>,
  ) {
    match event {
      // Trigger project save
      ProjectVariablesEvent::ProfilesChanged | ProjectVariablesEvent::VariablesChanged => {
        self.save(cx);
        cx.emit(ProjectEvent::ProjectDataChanged)
      }
      // Trigger workspace save
      ProjectVariablesEvent::ActiveProfile(_) => cx.emit(ProjectEvent::ProjectConfigChanged),
    }
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
        cx.emit(ProjectEvent::ProjectDataChanged)
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
        this.save_requested = false;
        let file = this.save_task_queued.take().map(|_| this.to_file(cx));
        cx.background_executor().spawn({
          let path = path.clone();
          async move {
            if let Some(bytes) = file {
              _ = bytes.save(&path);
            }
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
      // The queued could host an on-going serialization: save again once it's done
      self.save_requested = true;
      return;
    }
    let path = self.path.clone();
    self.save_task_queued = Some(cx.spawn(async move |this, cx| {
      cx.background_executor().timer(Duration::from_millis(200)).await;
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

      _ = this.update(cx, |this, cx| {
        this.save_task_queued.take();
        // Save the changes made while this save was running
        if std::mem::take(&mut this.save_requested) {
          this.save(cx);
        }
      });
    }));
  }
}

impl EventEmitter<ProjectEvent> for Project {}
