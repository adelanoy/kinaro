use crate::error::{ProjectError, ProjectResult};
use crate::{FileProjectMetadata, TestCase, TestStep, TestSuite};
use gpui_kit::{App, AppContext, ClipboardString, Context, Entity, EventEmitter, SharedString};
use ki_project::{FileTestMetadata, FileTestSuite, FileTestsContainer};
use ki_utils::{Offset, TestPath};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub mod step_delay;
pub mod step_rest;
pub mod test_case;
#[cfg(test)]
pub(crate) mod test_fixtures;
pub mod test_step;
pub mod test_suite;

/// Metadata shared by every node of the test tree (suite, case and step).
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TestMetadata {
  /// Path addressing the node in the test tree, its last segment being the node's id
  path: TestPath,
  /// Display name of the node
  pub(crate) name: SharedString,
  /// Optional markdown description of the node
  pub description: Option<SharedString>,
  /// Whether the node is excluded from runs
  pub disabled: bool,
}

impl TestMetadata {
  /// Returns the id of the node, i.e. the last segment of its [`path`](Self::path).
  #[inline]
  pub fn id(&self) -> Uuid {
    self.path.id()
  }

  /// Returns the path addressing the node in the test tree.
  pub fn path(&self) -> TestPath {
    self.path
  }

  /// Builds the metadata of a suite from its on-disk representation, keeping its id.
  pub(crate) fn suite_from_file(file_test_info: FileTestMetadata) -> Self {
    Self {
      path: TestPath::Suite(file_test_info.id),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  /// Builds the metadata of a suite from its on-disk representation, with a freshly generated id.
  pub(crate) fn new_suite_from_file(file_test_info: FileTestMetadata) -> Self {
    Self {
      path: TestPath::Suite(Uuid::new_v4()),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  /// Builds the metadata of a case of the suite `suite_id` from its on-disk representation,
  /// keeping its id.
  pub(crate) fn case_from_file(file_test_info: FileTestMetadata, suite_id: Uuid) -> Self {
    Self {
      path: TestPath::Case(suite_id, file_test_info.id),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  /// Builds the metadata of a case of the suite `suite_id` from its on-disk representation, with
  /// a freshly generated id.
  pub(crate) fn new_case_from_file(file_test_info: FileTestMetadata, suite_id: Uuid) -> Self {
    Self {
      path: TestPath::Case(suite_id, Uuid::new_v4()),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  /// Builds the metadata of a step of the case `case_id` (in the suite `suite_id`) from its
  /// on-disk representation, keeping its id.
  pub(crate) fn step_from_file(file_test_info: FileTestMetadata, suite_id: Uuid, case_id: Uuid) -> Self {
    Self {
      path: TestPath::Step(suite_id, case_id, file_test_info.id),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  /// Builds the metadata of a step of the case `case_id` (in the suite `suite_id`) from its
  /// on-disk representation, with a freshly generated id.
  pub(crate) fn new_step_from_file(file_test_info: FileTestMetadata, suite_id: Uuid, case_id: Uuid) -> Self {
    Self {
      path: TestPath::Step(suite_id, case_id, Uuid::new_v4()),
      name: file_test_info.name.into(),
      description: ki_utils::from_multiline(&file_test_info.description).map(|s| s.into()),
      disabled: file_test_info.disabled,
    }
  }

  /// Produces the on-disk representation of the metadata. The path is reduced to the node's id.
  pub(crate) fn to_file(&self) -> FileTestMetadata {
    FileTestMetadata {
      id: self.id(),
      name: self.name.to_string(),
      description: self
        .description
        .as_ref()
        .map(|s| ki_utils::to_multiline(s))
        .unwrap_or_default(),
      disabled: self.disabled,
    }
  }
}

/// A node of the test tree holding a [`TestMetadata`]: gives a uniform access to the metadata of
/// suites, cases and steps, so they can be renamed, described and enabled the same way.
pub trait TestMetadataOwner: Clone + 'static {
  /// Returns the id of the node, i.e. the last segment of its path.
  fn id(&self) -> Uuid;
  /// Returns the path addressing the node in the test tree.
  fn path(&self) -> TestPath;
  /// Returns the display name of the node.
  fn name(&self) -> SharedString;
  /// Renames the node. A no-op when `new_name` is its current name.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the name changed.
  fn set_name(&mut self, new_name: SharedString, cx: &mut Context<Self>)
  where
    Self: Sized;
  /// Returns the markdown description of the node, if any.
  fn description(&self) -> Option<SharedString>;
  /// Sets (or removes, with `None`) the description of the node. A no-op when it is unchanged.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container if the description changed.
  fn set_description(&mut self, new_description: Option<SharedString>, cx: &mut Context<Self>)
  where
    Self: Sized;
  /// Returns whether the node is excluded from runs.
  fn disabled(&self) -> bool;
  /// Enables the node if it is disabled, disables it otherwise.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] on the owning container.
  fn switch_disabled(&mut self, cx: &mut Context<Self>)
  where
    Self: Sized;
}

/// Events raised by the [`TestsContainer`] entity.
///
/// Test data events (`TestsModified`, `TestAdded`, `TestMoved`, `TestRemoved`)
/// mean the project file must be saved; configuration events
/// (`ConfigChanged`, `TestOpenInEditor`, `TestClosedInEditor`,
/// `EditorActiveTabIndex`) mean the workspace file must be saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestsContainerEvent {
  /// The content of the tests was modified (reordered, renamed, enabled/disabled, ...)
  TestsModified,
  /// A test was added at the given path (new or duplicated node)
  TestAdded(TestPath),
  /// A test was moved, changing its path: `(old path, new path)`
  TestMoved(TestPath, TestPath),
  /// A test was removed, along with all of its descendants
  TestRemoved(TestPath),
  /// The tests' configuration (e.g. expanded tree nodes) has changed
  ConfigChanged,
  /// A test was requested to be opened in the editor. The container's state
  /// is left unchanged: the editor is expected to handle the request and
  /// call [`TestsContainer::open_test_in_editor`]
  RequestOpenInEditor(TestPath),
  /// A test was opened in the editor
  TestOpenInEditor(TestPath),
  /// A test was closed in the editor
  TestClosedInEditor(TestPath),
  /// The editor's active tab index has changed
  EditorActiveTabIndex(Option<usize>),
}

/// The live test tree of a project, along with its UI configuration (expanded
/// tree nodes, tabs opened in the editor and the active one).
#[derive(Default, Debug, Eq, PartialEq)]
pub struct TestsContainer {
  /// The root test suites, in display order
  pub suites: Vec<Entity<TestSuite>>,
  opened_tree_nodes: HashSet<TestPath>,
  editor_tabs: Vec<TestPath>,
  active_editor_tab_index: Option<usize>,
}

impl TestsContainer {
  /// Returns the index of the active tab in [`TestsContainer::opened_editor_nodes`],
  /// or `None` when no tab is opened.
  #[inline]
  pub fn active_editor_tab_index(&self) -> Option<usize> {
    self.active_editor_tab_index
  }

  /// Adds a new, empty suite right after the suite `path` goes through, or
  /// at the end when `path` is `None` or its suite doesn't exist. Returns
  /// the path of the new suite.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] with the path of the new suite.
  pub fn add_test_suite(&mut self, path: Option<TestPath>, cx: &mut Context<Self>) -> TestPath {
    let position = match path {
      None => self.suites.len(),
      Some(path) => {
        let suite_id = path.suite_id();
        self
          .suites
          .iter()
          .position(|suite| suite.read(cx).meta.id() == suite_id)
          .map(|ix| min(ix + 1, self.suites.len()))
          .unwrap_or(self.suites.len())
      }
    };

    let name = ki_utils::next_available_name("New Suite", self.suites.iter().map(|suite| suite.read(cx).meta.name.clone()));
    let new_suite = TestSuite::new(name, cx.weak_entity());
    let path = new_suite.meta.path;
    self.suites.insert(position, cx.new(|_| new_suite));
    cx.emit(TestsContainerEvent::TestAdded(path));
    path
  }

  /// Returns the case `path` points to or goes through (the owning case of a
  /// step), or `None` for a suite path or when no such case exists.
  pub fn case_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestCase>> {
    self.suite_from_path(path, cx)?.read(cx).case_from_path(path, cx)
  }

  /// Removes `path` from the tabs opened in the editor. A no-op when it
  /// isn't opened. The active tab index is left unchanged: the caller is
  /// responsible for updating it through [`TestsContainer::set_active_editor_tab`].
  ///
  /// # Events
  /// [`TestsContainerEvent::TestClosedInEditor`] if the tab was removed.
  pub fn close_test_from_editor(&mut self, path: TestPath, cx: &mut Context<Self>) {
    if let Some(pos) = self.editor_tabs.iter().position(|p| *p == path) {
      self.editor_tabs.remove(pos);
      cx.emit(TestsContainerEvent::TestClosedInEditor(path));
    }
  }

  /// Marks the node at `id` as collapsed in the project tree.
  ///
  /// # Events
  /// [`TestsContainerEvent::ConfigChanged`]
  pub fn collapse_tree_node(&mut self, id: &TestPath, cx: &mut Context<Self>) {
    self.opened_tree_nodes.remove(id);
    cx.emit(TestsContainerEvent::ConfigChanged);
  }

  /// Duplicates the suite `path` goes through, with all of its cases and steps, and inserts the
  /// copy right after it. The copy gets fresh ids and a unique name. Returns the path of the copy.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no suite matches `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] with the path of the copy.
  pub fn duplicate_suite(&mut self, path: &TestPath, cx: &mut Context<Self>) -> ProjectResult<TestPath> {
    if let Some((ix, suite)) = self
      .suites
      .iter()
      .enumerate()
      .find(|(_, suite)| suite.read(cx).meta.id() == path.suite_id())
    {
      let new_name = ki_utils::next_available_name(
        &suite.read(cx).meta.name,
        self.suites.iter().map(|suite| suite.read(cx).meta.name.clone()),
      );
      let new_suite = suite.update(cx, |suite, cx| suite.duplicate(new_name, cx));
      let new_path = new_suite.meta.path;
      self.suites.insert(min(ix + 1, self.suites.len()), cx.new(|_| new_suite));
      cx.emit(TestsContainerEvent::TestAdded(new_path));
      Ok(new_path)
    } else {
      Err(ProjectError::TestNotFound(*path))
    }
  }

  /// Marks the node at `id` as expanded in the project tree.
  ///
  /// # Events
  /// [`TestsContainerEvent::ConfigChanged`]
  pub fn expand_tree_node(&mut self, id: TestPath, cx: &mut Context<Self>) {
    self.opened_tree_nodes.insert(id);
    cx.emit(TestsContainerEvent::ConfigChanged);
  }

  /// Builds the container from its on-disk representation and the
  /// workspace's saved configuration for this project. Expanded tree nodes
  /// and editor tabs that no longer address an existing node are dropped,
  /// and the active tab index is clamped to the remaining tabs.
  pub(super) fn from_file(file_container: FileTestsContainer, metadata: &FileProjectMetadata, cx: &mut Context<Self>) -> Self {
    let this_weak = cx.weak_entity();
    let mut this = Self {
      suites: file_container
        .suites
        .into_iter()
        .map(|file| cx.new(|cx| TestSuite::from_file(file, this_weak.clone(), false, cx)))
        .collect(),
      opened_tree_nodes: HashSet::new(),
      editor_tabs: vec![],
      active_editor_tab_index: None,
    };
    // Check config data: filter non-existing path and make sure the editor's tab index is consistent
    this.opened_tree_nodes = metadata
      .opened_tree_nodes
      .iter()
      .filter(|path| match path {
        TestPath::Suite(_) => this.suite_from_path(path, cx).is_some(),
        TestPath::Case(_, _) => this.case_from_path(path, cx).is_some(),
        TestPath::Step(_, _, _) => this.step_from_path(path, cx).is_some(),
      })
      .copied()
      .collect();

    this.editor_tabs = metadata
      .editor_tabs
      .iter()
      .filter(|path| match path {
        TestPath::Suite(_) => this.suite_from_path(path, cx).is_some(),
        TestPath::Case(_, _) => this.case_from_path(path, cx).is_some(),
        TestPath::Step(_, _, _) => this.step_from_path(path, cx).is_some(),
      })
      .copied()
      .collect();

    let mut active_editor_tab_index = metadata.active_editor_tab_index;
    if this.editor_tabs.is_empty() {
      active_editor_tab_index = None;
    } else if let Some(ix) = active_editor_tab_index
      && ix >= this.editor_tabs.len()
    {
      active_editor_tab_index = Some(this.editor_tabs.len() - 1);
    }
    this.active_editor_tab_index = active_editor_tab_index;

    this
  }

  /// Moves the node at `from` relative to the node at `to` (drag and drop in the project tree):
  /// - a suite takes the index the suite `to` had (so it lands after `to` when moved down, and
  ///   before it when moved up);
  /// - dropped on a suite, a case or a step is appended to that suite (a step becoming a case step);
  /// - dropped on a case step, a case or a step is placed after it (a step becoming a case step);
  /// - dropped on a multi-step case, a multi-step case is placed after it, while a case step or a
  ///   step is appended to its steps (a case step becoming a step);
  /// - dropped on a step, a case step or a step is placed after it in its case.
  ///
  /// Ids are kept, so only the paths of the moved node and its descendants change. Expanded tree
  /// nodes and editor tabs follow the moved nodes.
  ///
  /// # Errors
  /// - [`ProjectError::NoOp`] if `from` and `to` are the same, if a suite is dropped on a case or a
  ///   step, or if a multi-step case is dropped on a step.
  /// - [`ProjectError::TestNotFound`] if `from` or `to` doesn't address an existing node.
  ///
  /// # Events
  /// - [`TestsContainerEvent::ConfigChanged`] if an expanded tree node or an editor tab was moved.
  /// - [`TestsContainerEvent::TestMoved`] for each node whose path changed.
  /// - [`TestsContainerEvent::TestsModified`].
  pub fn move_test(&mut self, from: TestPath, to: TestPath, cx: &mut Context<Self>) -> ProjectResult<()> {
    if from == to || from.is_suite() && !to.is_suite() {
      return Err(ProjectError::NoOp);
    }
    if !self.path_exists(&from, cx) {
      return Err(ProjectError::TestNotFound(from));
    }
    if !self.path_exists(&to, cx) {
      return Err(ProjectError::TestNotFound(to));
    }
    let to_case_step = if let TestPath::Case(_, _) = to {
      self.case_from_path(&to, cx).unwrap().read(cx).is_case_step()
    } else {
      false
    };
    let from_case_step = if let TestPath::Case(_, _) = from {
      self.case_from_path(&from, cx).unwrap().read(cx).is_case_step()
    } else {
      false
    };

    let to_suite_level = match (from, to) {
      // Suites can only be moved among suites (checked above)
      (TestPath::Suite(_), _) => true,
      // Anything dropped on a suite or a case-step stays at (or is promoted to) suite level
      (_, TestPath::Suite(_)) => true,
      (_, TestPath::Case(_, _)) if to_case_step => true,
      // Dropped on a multi-case: a multi-case is placed after it, a case-step or a step goes into it
      (TestPath::Case(_, _), TestPath::Case(_, _)) => !from_case_step,
      (TestPath::Step(_, _, _), TestPath::Case(_, _)) => false,
      // Dropped on a step: a multi-case can't be nested in a case
      (TestPath::Case(_, _), TestPath::Step(_, _, _)) if !from_case_step => return Err(ProjectError::NoOp),
      (_, TestPath::Step(_, _, _)) => false,
    };

    let updated_paths = if to_suite_level {
      self.move_to_suite(from, to, cx)?
    } else {
      self.move_to_case(from, to, cx)?
    }
    .into_iter()
    .filter(|(old, new)| old != new)
    .collect::<HashMap<_, _>>();

    // Update the opened editor tabs and tree nodes states
    if !updated_paths.is_empty() {
      let mut config_changed = false;
      for path in &mut self.editor_tabs {
        if updated_paths.contains_key(path) {
          *path = *updated_paths.get(path).unwrap();
          config_changed = true;
        }
      }
      for (old_path, new_path) in &updated_paths {
        if self.opened_tree_nodes.contains(old_path) {
          self.opened_tree_nodes.remove(old_path);
          self.opened_tree_nodes.insert(*new_path);
          config_changed = true;
        }
      }
      if config_changed {
        cx.emit(TestsContainerEvent::ConfigChanged);
      }
    }
    // Notify all moves
    for (old_path, new_path) in updated_paths {
      cx.emit(TestsContainerEvent::TestMoved(old_path, new_path));
    }
    // Save project
    cx.emit(TestsContainerEvent::TestsModified);
    Ok(())
  }

  /// Moves `from` to the suite level, relative to `to`. Expects both paths to exist and `from` not
  /// to be a suite unless `to` is one. Returns the paths that changed, as `old path -> new path`.
  ///
  /// # Errors
  /// [`ProjectError::NoOp`] if the moved node can't be inserted in the suite of `to`.
  fn move_to_suite(
    &mut self,
    from: TestPath,
    to: TestPath,
    cx: &mut Context<Self>,
  ) -> ProjectResult<HashMap<TestPath, TestPath>> {
    let to_suite_ix = self
      .suites
      .iter()
      .position(|suite| suite.read(cx).path().is_parent(&to))
      .unwrap();
    match from {
      TestPath::Suite(_) => {
        let suite = self.remove_suite(&from, cx).unwrap();
        self.suites.insert(to_suite_ix, suite);
        Ok(HashMap::new())
      }
      TestPath::Case(_, _) => {
        let case = self
          .suite_from_path(&from, cx)
          .unwrap()
          .update(cx, |suite, cx| suite.remove_case(&from, cx).unwrap());
        self
          .suite_from_path(&to, cx)
          .unwrap()
          .update(cx, |suite, cx| suite.insert_case(case, &to, cx))
      }
      TestPath::Step(_, _, _) => {
        let weak_entity = cx.weak_entity();
        let case = self.case_from_path(&from, cx).unwrap().update(cx, |case, cx| {
          let step = case.remove_step(&from, cx).unwrap();
          let case = TestCase::from_step(step, to.suite_id(), weak_entity, cx);
          cx.new(|_| case)
        });
        let mut updated_paths = HashMap::new();
        updated_paths.insert(from, case.read(cx).path());
        self
          .suite_from_path(&to, cx)
          .unwrap()
          .update(cx, |suite, cx| suite.insert_case(case, &to, cx))?;
        Ok(updated_paths)
      }
    }
  }

  /// Moves the case step or step `from` into the multi-step case `to` goes through, relative to
  /// `to`. Expects both paths to exist. Returns the paths that changed, as `old path -> new path`.
  ///
  /// # Errors
  /// [`ProjectError::NoOp`] if `from` is a suite or the target case isn't a multi-step case.
  fn move_to_case(&mut self, from: TestPath, to: TestPath, cx: &mut Context<Self>) -> ProjectResult<HashMap<TestPath, TestPath>> {
    match from {
      TestPath::Suite(_) => Err(ProjectError::NoOp),
      TestPath::Case(_, _) => {
        // Pre-check ensured that 'from' is a step-case and 'to' is a multi-case: convert it on the fly and add
        let step = self.suite_from_path(&from, cx).unwrap().update(cx, |suite, cx| {
          let case = suite.remove_case(&from, cx).unwrap();
          let step = TestStep::from_case(case, &to, cx).unwrap();
          cx.new(|_| step)
        });
        let new_path = step.read(cx).path();
        self
          .case_from_path(&to, cx)
          .unwrap()
          .update(cx, |case, cx| case.insert_step(step, &to, cx))?;
        let mut updated_path = HashMap::new();
        updated_path.insert(from, new_path);
        Ok(updated_path)
      }
      TestPath::Step(_, _, _) => {
        // Pre-check ensured that 'to' is a multi-case: extract step and add
        let step = self
          .case_from_path(&from, cx)
          .unwrap()
          .update(cx, |case, cx| case.remove_step(&from, cx).unwrap());
        let new_path = self
          .case_from_path(&to, cx)
          .unwrap()
          .update(cx, |case, cx| case.insert_step(step, &to, cx))?;
        let mut updated_paths = HashMap::new();
        updated_paths.insert(from, new_path);
        Ok(updated_paths)
      }
    }
  }

  /// Moves the suite at `path` one position down ([`Offset::Plus`]) or up ([`Offset::Minus`]). A
  /// no-op when it is already the last, or the first, suite.
  ///
  /// # Errors
  /// [`ProjectError::TestNotFound`] if no suite is at `path`.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestsModified`] if the suite moved.
  pub fn offset_suite(&mut self, path: TestPath, offset: Offset, cx: &mut Context<Self>) -> ProjectResult<()> {
    if let Some(pos) = self.suites.iter().position(|suite| suite.read(cx).path() == path) {
      let new_pos = match offset {
        Offset::Plus => min(pos + 1, self.suites.len() - 1),
        Offset::Minus => {
          if pos > 0 {
            pos - 1
          } else {
            pos
          }
        }
      };
      if pos != new_pos {
        self.suites.swap(pos, new_pos);
        cx.emit(TestsContainerEvent::TestsModified);
      }
      Ok(())
    } else {
      Err(ProjectError::TestNotFound(path))
    }
  }

  /// Adds `path` to the tabs opened in the editor (a no-op if it already is)
  /// and sets `selected_tab` as the active tab index.
  ///
  /// # Events
  /// - [`TestsContainerEvent::TestOpenInEditor`] if `path` wasn't opened yet.
  /// - [`TestsContainerEvent::EditorActiveTabIndex`] if the active tab index
  ///   changed.
  pub fn open_test_in_editor(&mut self, path: TestPath, selected_tab: Option<usize>, cx: &mut Context<Self>) {
    if !self.editor_tabs.contains(&path) {
      self.editor_tabs.push(path);
      cx.emit(TestsContainerEvent::TestOpenInEditor(path));
    }
    if self.active_editor_tab_index != selected_tab {
      self.active_editor_tab_index = selected_tab;
      cx.emit(TestsContainerEvent::EditorActiveTabIndex(selected_tab));
    }
  }

  /// Returns the paths of the nodes expanded in the project tree.
  #[inline]
  pub fn opened_tree_nodes(&self) -> &HashSet<TestPath> {
    &self.opened_tree_nodes
  }

  /// Returns the paths of the tabs opened in the editor, in display order.
  #[inline]
  pub fn opened_editor_nodes(&self) -> &[TestPath] {
    &self.editor_tabs
  }

  /// Pastes the suite held in `cb_str` (a serialized suite) right after the suite at `target`. The
  /// pasted suite and its descendants get fresh ids. Nothing is pasted, and a warning is logged, if
  /// `target` isn't a suite or the clipboard doesn't hold a valid suite.
  ///
  /// # Events
  /// [`TestsContainerEvent::TestAdded`] with the path of the pasted suite.
  pub fn paste_from_clipboard(&mut self, cb_str: &ClipboardString, target: TestPath, cx: &mut Context<Self>) {
    let Some(ix) = self
      .suites
      .iter()
      .position(|suite| suite.read(cx).meta.path == target)
      .map(|ix| min(ix + 1, self.suites.len()))
    else {
      warn!("TestsContainer::paste_from_clipboard: Attempted to paste on a unknown path: {target}");
      return;
    };
    let Ok(suite) = serde_json::from_str::<FileTestSuite>(&cb_str.text) else {
      warn!("Could not paste Suite: Clipboard contains invalid Json");
      debug!("Invalid Json: {}", cb_str.text);
      return;
    };
    let this_weak = cx.weak_entity();
    let suite = cx.new(|cx| TestSuite::from_file(suite, this_weak, true, cx));
    let new_path = suite.read(cx).meta.path;
    self.suites.insert(ix, suite);
    cx.emit(TestsContainerEvent::TestAdded(new_path));
  }

  /// Returns whether `path` addresses an existing node.
  pub fn path_exists(&self, path: &TestPath, cx: &App) -> bool {
    match path {
      TestPath::Suite(_) => self.suite_from_path(path, cx).is_some(),
      TestPath::Case(_, _) => self.case_from_path(path, cx).is_some(),
      TestPath::Step(_, _, _) => self.step_from_path(path, cx).is_some(),
    }
  }

  /// Removes the node at `path` along with all of its descendants, dropping them from the
  /// expanded tree nodes and the editor tabs. A no-op when no such node exists. The active tab
  /// index is left unchanged.
  ///
  /// # Events
  /// - [`TestsContainerEvent::TestRemoved`] for the removed node and each of its descendants.
  /// - [`TestsContainerEvent::ConfigChanged`] if an expanded tree node or an editor tab was dropped.
  pub fn remove_test(&mut self, path: &TestPath, cx: &mut Context<Self>) {
    let removed_paths = match path {
      TestPath::Suite(_) => self.remove_suite(path, cx).map(|suite| suite.read(cx).all_paths(cx)),
      TestPath::Case(_, _) => self
        .suite_from_path(path, cx)
        .and_then(|suite| suite.update(cx, |suite, cx| suite.remove_case(path, cx)))
        .map(|case| case.read(cx).all_paths(cx)),
      TestPath::Step(_, _, _) => self
        .case_from_path(path, cx)
        .and_then(|case| case.update(cx, |case, cx| case.remove_step(path, cx)))
        .map(|step| vec![step.read(cx).path()]),
    }
    .unwrap_or(vec![]);
    let mut config_changed = false;
    for path in removed_paths {
      config_changed |= self.opened_tree_nodes.remove(&path);
      if let Some(ix) = self.editor_tabs.iter().position(|tab_path| tab_path == &path) {
        self.editor_tabs.remove(ix);
        config_changed = true;
      }
      cx.emit(TestsContainerEvent::TestRemoved(path));
    }
    if config_changed {
      cx.emit(TestsContainerEvent::ConfigChanged);
    }
  }

  /// Removes the suite at `path` and returns it, or `None` when no suite is at `path`.
  fn remove_suite(&mut self, path: &TestPath, cx: &mut Context<Self>) -> Option<Entity<TestSuite>> {
    let ix = self.suites.iter().position(|case| case.read(cx).path() == *path)?;
    let suite = self.suites.remove(ix);
    Some(suite)
  }

  /// Requests the node at `path` to be opened in the editor, without
  /// changing the container's state.
  ///
  /// # Events
  /// [`TestsContainerEvent::RequestOpenInEditor`] with `path`.
  pub fn request_open_in_editor(&mut self, path: TestPath, cx: &mut Context<Self>) {
    cx.emit(TestsContainerEvent::RequestOpenInEditor(path));
  }

  /// Sets the index of the active tab in the editor. The index isn't checked
  /// against the opened tabs.
  ///
  /// # Events
  /// [`TestsContainerEvent::EditorActiveTabIndex`] if the index changed.
  #[inline]
  pub fn set_active_editor_tab(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
    if self.active_editor_tab_index != ix {
      self.active_editor_tab_index = ix;
      cx.emit(TestsContainerEvent::EditorActiveTabIndex(ix));
    }
  }

  /// Returns the step `path` points to, or `None` for a suite or case path
  /// or when no such step exists.
  pub fn step_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestStep>> {
    self.case_from_path(path, cx)?.read(cx).step_from_path(path, cx)
  }

  /// Returns the suite `path` points to or goes through, or `None` when no
  /// such suite exists.
  pub fn suite_from_path(&self, path: &TestPath, cx: &App) -> Option<Entity<TestSuite>> {
    self
      .suites
      .iter()
      .find(|suite| suite.read(cx).meta.path.is_parent(path))
      .cloned()
  }

  /// Produces the on-disk representation of the test tree. The UI
  /// configuration isn't part of it: it's saved in the workspace file.
  pub fn to_file(&self, cx: &App) -> FileTestsContainer {
    FileTestsContainer {
      suites: self.suites.iter().map(|suite| suite.read(cx).get_file(cx)).collect(),
    }
  }
}

impl EventEmitter<TestsContainerEvent> for TestsContainer {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::step_delay::DelayStep;
  use crate::test::test_case::TestCaseType;
  use crate::test::test_fixtures::*;
  use crate::test::test_step::StepData;
  use gpui_kit::TestAppContext;
  use ki_project::FileTestCaseType;
  use ki_utils::SUITE;

  // TestMetadata

  #[test]
  fn metadata_from_file_keeps_ids() {
    let (suite_id, case_id, step_id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    assert_eq!(
      TestMetadata::suite_from_file(file_meta(suite_id, "s")).path(),
      TestPath::Suite(suite_id)
    );
    assert_eq!(
      TestMetadata::case_from_file(file_meta(case_id, "c"), suite_id).path(),
      TestPath::Case(suite_id, case_id)
    );
    assert_eq!(
      TestMetadata::step_from_file(file_meta(step_id, "s"), suite_id, case_id).path(),
      TestPath::Step(suite_id, case_id, step_id)
    );
  }

  #[test]
  fn metadata_new_from_file_generates_ids() {
    let (suite_id, case_id, step_id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let suite = TestMetadata::new_suite_from_file(file_meta(suite_id, "s"));
    assert!(suite.path().is_suite());
    assert_ne!(suite.id(), suite_id);
    let case = TestMetadata::new_case_from_file(file_meta(case_id, "c"), suite_id);
    assert!(matches!(case.path(), TestPath::Case(suite, case) if suite == suite_id && case != case_id));
    let step = TestMetadata::new_step_from_file(file_meta(step_id, "s"), suite_id, case_id);
    assert!(matches!(step.path(), TestPath::Step(suite, case, step) if suite == suite_id && case == case_id && step != step_id));
  }

  #[test]
  fn metadata_file_round_trip() {
    let id = Uuid::new_v4();
    let file = FileTestMetadata {
      id,
      name: "name".to_string(),
      description: vec!["line 1".to_string(), "line 2".to_string()],
      disabled: true,
    };
    let meta = TestMetadata::suite_from_file(file);
    assert_eq!(meta.name, "name");
    assert_eq!(meta.description.as_deref(), Some("line 1\nline 2"));
    assert!(meta.disabled);
    let file = meta.to_file();
    assert_eq!(file.id, id);
    assert_eq!(file.name, "name");
    assert_eq!(file.description, vec!["line 1", "line 2"]);
    assert!(file.disabled);
  }

  #[test]
  fn metadata_without_description() {
    let meta = TestMetadata::suite_from_file(file_meta(Uuid::new_v4(), "s"));
    assert_eq!(meta.description, None);
    assert!(meta.to_file().description.is_empty());
  }

  // Loading and saving

  #[gpui_kit::test]
  fn from_file_builds_the_tree(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    assert_eq!(
      all_paths(&tests, cx),
      vec![
        p.suite_a,
        p.case_a,
        p.step_a1,
        p.step_a2,
        p.case_step_a,
        p.suite_b,
        p.case_b,
        p.step_b1,
        p.case_step_b
      ]
    );
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite B"]);
    assert!(case(&tests, p.case_step_a, cx).read_with(cx, |case, _| case.is_case_step()));
    assert!(!case(&tests, p.case_a, cx).read_with(cx, |case, _| case.is_case_step()));
    tests.read_with(cx, |tests, _| {
      assert!(tests.opened_tree_nodes().is_empty());
      assert!(tests.opened_editor_nodes().is_empty());
      assert_eq!(tests.active_editor_tab_index(), None);
    });
  }

  #[gpui_kit::test]
  fn from_file_keeps_existing_config_paths(cx: &mut TestAppContext) {
    let p = Paths::new();
    let config = FileProjectMetadata {
      opened_tree_nodes: [p.suite_a, p.case_a].into_iter().collect(),
      editor_tabs: vec![p.step_a1, p.suite_b, p.case_step_b],
      active_editor_tab_index: Some(1),
      ..Default::default()
    };
    let tests = load(p.file(), config, cx);
    tests.read_with(cx, |tests, _| {
      assert_eq!(*tests.opened_tree_nodes(), [p.suite_a, p.case_a].into_iter().collect());
      assert_eq!(tests.opened_editor_nodes(), [p.step_a1, p.suite_b, p.case_step_b]);
      assert_eq!(tests.active_editor_tab_index(), Some(1));
    });
  }

  #[gpui_kit::test]
  fn from_file_drops_unknown_config_paths_and_clamps_active_tab(cx: &mut TestAppContext) {
    let p = Paths::new();
    let unknown_case = TestPath::Case(p.suite_a.id(), Uuid::new_v4());
    let unknown_step = TestPath::Step(p.suite_a.id(), p.case_a.id(), Uuid::new_v4());
    let config = FileProjectMetadata {
      opened_tree_nodes: [p.suite_a, unknown_case, TestPath::Suite(Uuid::new_v4())]
        .into_iter()
        .collect(),
      editor_tabs: vec![unknown_step, p.case_a, unknown_case],
      active_editor_tab_index: Some(2),
      ..Default::default()
    };
    let tests = load(p.file(), config, cx);
    tests.read_with(cx, |tests, _| {
      assert_eq!(*tests.opened_tree_nodes(), [p.suite_a].into_iter().collect());
      assert_eq!(tests.opened_editor_nodes(), [p.case_a]);
      assert_eq!(tests.active_editor_tab_index(), Some(0));
    });
  }

  #[gpui_kit::test]
  fn from_file_without_tabs_has_no_active_tab(cx: &mut TestAppContext) {
    let p = Paths::new();
    let config = FileProjectMetadata {
      editor_tabs: vec![TestPath::Suite(Uuid::new_v4())],
      active_editor_tab_index: Some(0),
      ..Default::default()
    };
    let tests = load(p.file(), config, cx);
    tests.read_with(cx, |tests, _| assert_eq!(tests.active_editor_tab_index(), None));
  }

  #[gpui_kit::test]
  fn to_file_round_trips(cx: &mut TestAppContext) {
    let p = Paths::new();
    let file = p.file();
    let tests = load(file.clone(), FileProjectMetadata::default(), cx);
    let saved = cx.read(|cx| tests.read(cx).to_file(cx));
    assert_eq!(serde_json::to_value(saved).unwrap(), serde_json::to_value(file).unwrap());
  }

  // Lookups

  #[gpui_kit::test]
  fn lookups_resolve_nodes_and_their_parents(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    cx.read(|cx| {
      let tests = tests.read(cx);
      let suite_path = |path: &TestPath| tests.suite_from_path(path, cx).map(|suite| suite.read(cx).path());
      let case_path = |path: &TestPath| tests.case_from_path(path, cx).map(|case| case.read(cx).path());
      let step_path = |path: &TestPath| tests.step_from_path(path, cx).map(|step| step.read(cx).path());
      assert_eq!(suite_path(&p.suite_b), Some(p.suite_b));
      assert_eq!(suite_path(&p.case_b), Some(p.suite_b));
      assert_eq!(suite_path(&p.step_a2), Some(p.suite_a));
      assert_eq!(case_path(&p.suite_a), None);
      assert_eq!(case_path(&p.case_step_a), Some(p.case_step_a));
      assert_eq!(case_path(&p.step_a2), Some(p.case_a));
      assert_eq!(step_path(&p.case_a), None);
      assert_eq!(step_path(&p.step_b1), Some(p.step_b1));
    });
  }

  #[gpui_kit::test]
  fn lookups_of_unknown_paths(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let unknown_suite = TestPath::Suite(Uuid::new_v4());
    let unknown_case = TestPath::Case(p.suite_a.id(), Uuid::new_v4());
    let unknown_step = TestPath::Step(p.suite_a.id(), p.case_a.id(), Uuid::new_v4());
    // A step addressed through the wrong case
    let misplaced_step = TestPath::Step(p.suite_a.id(), p.case_b.id(), p.step_b1.id());
    cx.read(|cx| {
      let tests = tests.read(cx);
      assert!(tests.suite_from_path(&unknown_suite, cx).is_none());
      assert!(tests.case_from_path(&unknown_case, cx).is_none());
      assert!(tests.step_from_path(&unknown_step, cx).is_none());
      assert!(tests.step_from_path(&misplaced_step, cx).is_none());
      for path in [unknown_suite, unknown_case, unknown_step, misplaced_step] {
        assert!(!tests.path_exists(&path, cx), "{path} shouldn't exist");
      }
      for path in [p.suite_a, p.case_a, p.step_a1, p.case_step_b] {
        assert!(tests.path_exists(&path, cx), "{path} should exist");
      }
    });
  }

  // Adding suites

  #[gpui_kit::test]
  fn add_test_suite_without_path_appends(cx: &mut TestAppContext) {
    let (_, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let new_path = tests.update(cx, |tests, cx| tests.add_test_suite(None, cx));
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite B", "New Suite"]);
    assert_eq!(all_paths(&tests, cx).last(), Some(&new_path));
    assert!(case_names(&tests, new_path, cx).is_empty());
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(new_path)]);
  }

  #[gpui_kit::test]
  fn add_test_suite_inserts_after_the_suite_of_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    tests.update(cx, |tests, cx| tests.add_test_suite(Some(p.step_a1), cx));
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "New Suite", "Suite B"]);
  }

  #[gpui_kit::test]
  fn add_test_suite_with_unknown_path_appends(cx: &mut TestAppContext) {
    let (_, tests) = fixture(cx);
    tests.update(cx, |tests, cx| {
      tests.add_test_suite(Some(TestPath::Suite(Uuid::new_v4())), cx)
    });
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite B", "New Suite"]);
  }

  #[gpui_kit::test]
  fn add_test_suite_makes_names_unique(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    for _ in 0..3 {
      tests.update(cx, |tests, cx| tests.add_test_suite(None, cx));
    }
    assert_eq!(suite_names(&tests, cx), vec!["New Suite", "New Suite_1", "New Suite_2"]);
  }

  // Removing

  #[gpui_kit::test]
  fn remove_test_suite_removes_descendants_and_their_config(cx: &mut TestAppContext) {
    let p = Paths::new();
    let config = FileProjectMetadata {
      opened_tree_nodes: [p.suite_a, p.case_a, p.suite_b].into_iter().collect(),
      editor_tabs: vec![p.suite_b, p.step_a1],
      ..Default::default()
    };
    let tests = load(p.file(), config, cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| tests.remove_test(&p.suite_a, cx));

    assert_eq!(all_paths(&tests, cx), vec![p.suite_b, p.case_b, p.step_b1, p.case_step_b]);
    tests.read_with(cx, |tests, _| {
      assert_eq!(*tests.opened_tree_nodes(), [p.suite_b].into_iter().collect());
      assert_eq!(tests.opened_editor_nodes(), [p.suite_b]);
    });
    assert_eq!(
      *events.borrow(),
      vec![
        TestsContainerEvent::TestRemoved(p.suite_a),
        TestsContainerEvent::TestRemoved(p.case_a),
        TestsContainerEvent::TestRemoved(p.step_a1),
        TestsContainerEvent::TestRemoved(p.step_a2),
        TestsContainerEvent::TestRemoved(p.case_step_a),
        TestsContainerEvent::ConfigChanged,
      ]
    );
  }

  #[gpui_kit::test]
  fn remove_test_case_removes_its_steps(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| tests.remove_test(&p.case_a, cx));

    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case step A"]);
    // Nothing was opened: the configuration is unchanged
    assert_eq!(
      *events.borrow(),
      vec![
        TestsContainerEvent::TestRemoved(p.case_a),
        TestsContainerEvent::TestRemoved(p.step_a1),
        TestsContainerEvent::TestRemoved(p.step_a2),
      ]
    );
  }

  #[gpui_kit::test]
  fn remove_test_case_step(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    tests.update(cx, |tests, cx| tests.remove_test(&p.case_step_b, cx));
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B"]);
  }

  #[gpui_kit::test]
  fn remove_test_step(cx: &mut TestAppContext) {
    let p = Paths::new();
    let config = FileProjectMetadata {
      editor_tabs: vec![p.step_a1],
      ..Default::default()
    };
    let tests = load(p.file(), config, cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| tests.remove_test(&p.step_a1, cx));

    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A2"]);
    tests.read_with(cx, |tests, _| assert!(tests.opened_editor_nodes().is_empty()));
    assert_eq!(
      *events.borrow(),
      vec![
        TestsContainerEvent::TestRemoved(p.step_a1),
        TestsContainerEvent::ConfigChanged
      ]
    );
  }

  #[gpui_kit::test]
  fn remove_test_unknown_path_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let before = all_paths(&tests, cx);
    let events = record_events(&tests, cx);
    for path in [
      TestPath::Suite(Uuid::new_v4()),
      TestPath::Case(p.suite_a.id(), Uuid::new_v4()),
      TestPath::Step(p.suite_a.id(), p.case_a.id(), Uuid::new_v4()),
    ] {
      tests.update(cx, |tests, cx| tests.remove_test(&path, cx));
    }
    assert_eq!(all_paths(&tests, cx), before);
    assert!(events.borrow().is_empty());
  }

  // Duplicating suites

  #[gpui_kit::test]
  fn duplicate_suite_inserts_a_deep_copy_after_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    let copy = tests.update(cx, |tests, cx| tests.duplicate_suite(&p.case_a, cx)).unwrap();

    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite A_1", "Suite B"]);
    assert_eq!(case_names(&tests, copy, cx), vec!["Case A", "Case step A"]);
    let copy_paths = subtree(&tests, copy, cx);
    assert_eq!(copy_paths.len(), 5);
    let original_paths = subtree(&tests, p.suite_a, cx);
    // Every node of the copy has a fresh id and lives in the copy
    for path in &copy_paths {
      assert!(copy.is_parent(path));
      assert!(original_paths.iter().all(|original| original.id() != path.id()));
    }
    let copied_case = copy_paths[1];
    assert_eq!(step_names(&tests, copied_case, cx), vec!["Step A1", "Step A2"]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(copy)]);
  }

  #[gpui_kit::test]
  fn duplicate_suite_unknown_is_not_found(cx: &mut TestAppContext) {
    let (_, tests) = fixture(cx);
    let unknown = TestPath::Suite(Uuid::new_v4());
    let result = tests.update(cx, |tests, cx| tests.duplicate_suite(&unknown, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == unknown));
  }

  // UI configuration

  #[gpui_kit::test]
  fn expand_and_collapse_tree_nodes(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| {
      tests.expand_tree_node(p.suite_a, cx);
      tests.expand_tree_node(p.case_a, cx);
      tests.collapse_tree_node(&p.suite_a, cx);
    });
    tests.read_with(cx, |tests, _| {
      assert_eq!(*tests.opened_tree_nodes(), [p.case_a].into_iter().collect())
    });
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::ConfigChanged; 3]);
  }

  #[gpui_kit::test]
  fn open_test_in_editor_adds_the_tab_once(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| {
      tests.open_test_in_editor(p.case_a, Some(0), cx);
      tests.open_test_in_editor(p.step_b1, Some(1), cx);
      tests.open_test_in_editor(p.case_a, Some(0), cx);
    });
    tests.read_with(cx, |tests, _| {
      assert_eq!(tests.opened_editor_nodes(), [p.case_a, p.step_b1]);
      assert_eq!(tests.active_editor_tab_index(), Some(0));
    });
    assert_eq!(
      *events.borrow(),
      vec![
        TestsContainerEvent::TestOpenInEditor(p.case_a),
        TestsContainerEvent::EditorActiveTabIndex(Some(0)),
        TestsContainerEvent::TestOpenInEditor(p.step_b1),
        TestsContainerEvent::EditorActiveTabIndex(Some(1)),
        TestsContainerEvent::EditorActiveTabIndex(Some(0)),
      ]
    );
  }

  #[gpui_kit::test]
  fn close_test_from_editor_removes_the_tab(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    tests.update(cx, |tests, cx| {
      tests.open_test_in_editor(p.case_a, Some(0), cx);
      tests.open_test_in_editor(p.step_b1, Some(1), cx);
    });
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| {
      tests.close_test_from_editor(p.case_a, cx);
      // Not opened: no-op
      tests.close_test_from_editor(p.suite_a, cx);
    });
    tests.read_with(cx, |tests, _| {
      assert_eq!(tests.opened_editor_nodes(), [p.step_b1]);
      // Left for the caller to update
      assert_eq!(tests.active_editor_tab_index(), Some(1));
    });
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestClosedInEditor(p.case_a)]);
  }

  #[gpui_kit::test]
  fn set_active_editor_tab_emits_on_change_only(cx: &mut TestAppContext) {
    let (_, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| {
      tests.set_active_editor_tab(Some(2), cx);
      tests.set_active_editor_tab(Some(2), cx);
      tests.set_active_editor_tab(None, cx);
    });
    tests.read_with(cx, |tests, _| assert_eq!(tests.active_editor_tab_index(), None));
    assert_eq!(
      *events.borrow(),
      vec![
        TestsContainerEvent::EditorActiveTabIndex(Some(2)),
        TestsContainerEvent::EditorActiveTabIndex(None)
      ]
    );
  }

  #[gpui_kit::test]
  fn request_open_in_editor_only_emits(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| tests.request_open_in_editor(p.case_a, cx));
    tests.read_with(cx, |tests, _| assert!(tests.opened_editor_nodes().is_empty()));
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::RequestOpenInEditor(p.case_a)]);
  }

  // Offsetting suites

  #[gpui_kit::test]
  fn offset_suite_swaps_with_its_neighbour(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests
      .update(cx, |tests, cx| tests.offset_suite(p.suite_a, Offset::Plus, cx))
      .unwrap();
    assert_eq!(suite_names(&tests, cx), vec!["Suite B", "Suite A"]);
    tests
      .update(cx, |tests, cx| tests.offset_suite(p.suite_a, Offset::Minus, cx))
      .unwrap();
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite B"]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 2]);
  }

  #[gpui_kit::test]
  fn offset_suite_at_the_bounds_is_a_no_op(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests
      .update(cx, |tests, cx| tests.offset_suite(p.suite_a, Offset::Minus, cx))
      .unwrap();
    tests
      .update(cx, |tests, cx| tests.offset_suite(p.suite_b, Offset::Plus, cx))
      .unwrap();
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite B"]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn offset_suite_unknown_is_not_found(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let result = tests.update(cx, |tests, cx| tests.offset_suite(p.case_a, Offset::Plus, cx));
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == p.case_a));
  }

  // Pasting suites

  fn copied_suite(p: &Paths) -> ClipboardString {
    clipboard(serde_json::to_string(&p.file().suites[0]).unwrap(), SUITE)
  }

  #[gpui_kit::test]
  fn paste_suite_inserts_a_copy_with_fresh_ids_after_target(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| tests.paste_from_clipboard(&copied_suite(&p), p.suite_a, cx));

    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite A", "Suite B"]);
    let pasted = cx.read(|cx| tests.read(cx).suites[1].read(cx).all_paths(cx));
    assert_eq!(pasted.len(), 5);
    let original_ids = subtree(&tests, p.suite_a, cx);
    for path in &pasted {
      assert!(pasted[0].is_parent(path));
      assert!(original_ids.iter().all(|original| original.id() != path.id()));
    }
    assert_eq!(step_names(&tests, pasted[1], cx), vec!["Step A1", "Step A2"]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestAdded(pasted[0])]);
  }

  #[gpui_kit::test]
  fn paste_suite_on_a_non_suite_or_invalid_content_is_ignored(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let events = record_events(&tests, cx);
    tests.update(cx, |tests, cx| {
      tests.paste_from_clipboard(&copied_suite(&p), p.case_a, cx);
      tests.paste_from_clipboard(&clipboard("not json".to_string(), SUITE), p.suite_a, cx);
    });
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite B"]);
    assert!(events.borrow().is_empty());
  }

  // Moving

  /// Moves `from` to `to`, returning the result and the emitted events.
  fn move_test(
    tests: &Entity<TestsContainer>,
    from: TestPath,
    to: TestPath,
    cx: &mut TestAppContext,
  ) -> (ProjectResult<()>, Vec<TestsContainerEvent>) {
    let events = record_events(tests, cx);
    let result = tests.update(cx, |tests, cx| tests.move_test(from, to, cx));
    let events = events.borrow().clone();
    (result, events)
  }

  #[gpui_kit::test]
  fn move_test_rejects_invalid_moves(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let before = all_paths(&tests, cx);
    let unknown = TestPath::Case(p.suite_a.id(), Uuid::new_v4());
    for (from, to) in [
      (p.case_a, p.case_a),
      (p.suite_a, p.case_b),
      (p.suite_a, p.step_b1),
      (p.case_a, p.step_b1),
    ] {
      let (result, events) = move_test(&tests, from, to, cx);
      assert!(matches!(result, Err(ProjectError::NoOp)), "{from} -> {to}");
      assert!(events.is_empty());
    }
    let (result, _) = move_test(&tests, unknown, p.case_b, cx);
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == unknown));
    let (result, _) = move_test(&tests, p.case_b, unknown, cx);
    assert!(matches!(result, Err(ProjectError::TestNotFound(path)) if path == unknown));
    assert_eq!(all_paths(&tests, cx), before);
  }

  #[gpui_kit::test]
  fn move_suite_takes_the_index_of_the_target(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    tests.update(cx, |tests, cx| tests.add_test_suite(None, cx));
    let (result, events) = move_test(&tests, p.suite_a, p.suite_b, cx);
    result.unwrap();
    assert_eq!(suite_names(&tests, cx), vec!["Suite B", "Suite A", "New Suite"]);
    // Paths of suites don't depend on their position
    assert_eq!(events, vec![TestsContainerEvent::TestsModified]);

    move_test(&tests, p.suite_a, p.suite_b, cx).0.unwrap();
    assert_eq!(suite_names(&tests, cx), vec!["Suite A", "Suite B", "New Suite"]);
  }

  #[gpui_kit::test]
  fn move_multi_case_on_a_suite_appends_it_with_its_steps(cx: &mut TestAppContext) {
    let p = Paths::new();
    let config = FileProjectMetadata {
      opened_tree_nodes: [p.case_a].into_iter().collect(),
      editor_tabs: vec![p.suite_a, p.step_a2],
      ..Default::default()
    };
    let tests = load(p.file(), config, cx);
    let (result, events) = move_test(&tests, p.case_a, p.suite_b, cx);
    result.unwrap();

    let sb = p.suite_b.id();
    let new_case = TestPath::Case(sb, p.case_a.id());
    let new_step_a1 = TestPath::Step(sb, p.case_a.id(), p.step_a1.id());
    let new_step_a2 = TestPath::Step(sb, p.case_a.id(), p.step_a2.id());
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case step A"]);
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Case step B", "Case A"]);
    assert_eq!(step_names(&tests, new_case, cx), vec!["Step A1", "Step A2"]);
    assert_eq!(name_at(&tests, new_step_a2, cx), "Step A2");
    tests.read_with(cx, |tests, _| {
      assert_eq!(*tests.opened_tree_nodes(), [new_case].into_iter().collect());
      assert_eq!(tests.opened_editor_nodes(), [p.suite_a, new_step_a2]);
    });
    assert_eq!(events.first(), Some(&TestsContainerEvent::ConfigChanged));
    assert_eq!(events.last(), Some(&TestsContainerEvent::TestsModified));
    // Moves are notified in no particular order
    let moves = &events[1..events.len() - 1];
    assert_eq!(moves.len(), 3);
    for expected in [
      TestsContainerEvent::TestMoved(p.case_a, new_case),
      TestsContainerEvent::TestMoved(p.step_a1, new_step_a1),
      TestsContainerEvent::TestMoved(p.step_a2, new_step_a2),
    ] {
      assert!(moves.contains(&expected), "missing {expected:?}");
    }
  }

  #[gpui_kit::test]
  fn move_multi_case_on_a_multi_case_places_it_after(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    move_test(&tests, p.case_a, p.case_b, cx).0.unwrap();
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Case A", "Case step B"]);
  }

  #[gpui_kit::test]
  fn move_multi_case_within_its_suite_keeps_its_path(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let (result, events) = move_test(&tests, p.case_a, p.case_step_a, cx);
    result.unwrap();
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case step A", "Case A"]);
    assert_eq!(events, vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn move_case_step_on_a_case_step_places_it_after(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let (result, events) = move_test(&tests, p.case_step_a, p.case_b, cx);
    result.unwrap();
    // Dropped on a multi-case: becomes its last step, keeping its id
    let new_step = TestPath::Step(p.suite_b.id(), p.case_b.id(), p.case_step_a.id());
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case A"]);
    assert_eq!(step_names(&tests, p.case_b, cx), vec!["Step B1", "Case step A"]);
    assert!(step(&tests, new_step, cx).read_with(cx, |step, _| step.data == StepData::Delay(DelayStep(300))));
    assert_eq!(
      events,
      vec![
        TestsContainerEvent::TestMoved(p.case_step_a, new_step),
        TestsContainerEvent::TestsModified
      ]
    );

    move_test(&tests, new_step, p.case_step_b, cx).0.unwrap();
    // Dropped on a case step: becomes a case step again, right after it
    assert_eq!(
      case_names(&tests, p.suite_b, cx),
      vec!["Case B", "Case step B", "Case step A"]
    );
    let back = case(&tests, TestPath::Case(p.suite_b.id(), p.case_step_a.id()), cx);
    back.read_with(cx, |case, _| {
      assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(300))))
    });
  }

  #[gpui_kit::test]
  fn move_case_step_on_a_case_step_of_another_suite(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    move_test(&tests, p.case_step_a, p.case_step_b, cx).0.unwrap();
    assert_eq!(
      case_names(&tests, p.suite_b, cx),
      vec!["Case B", "Case step B", "Case step A"]
    );
  }

  #[gpui_kit::test]
  fn move_case_step_on_a_step_places_it_after(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    move_test(&tests, p.case_step_b, p.step_a1, cx).0.unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Case step B", "Step A2"]);
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B"]);
  }

  #[gpui_kit::test]
  fn move_step_on_a_suite_promotes_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let (result, events) = move_test(&tests, p.step_a1, p.suite_b, cx);
    result.unwrap();
    let new_case = TestPath::Case(p.suite_b.id(), p.step_a1.id());
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A2"]);
    assert_eq!(case_names(&tests, p.suite_b, cx), vec!["Case B", "Case step B", "Step A1"]);
    case(&tests, new_case, cx).read_with(cx, |case, _| {
      assert!(matches!(case.case_type(), TestCaseType::CaseStep { data } if *data == StepData::Delay(DelayStep(100))))
    });
    assert_eq!(
      events,
      vec![
        TestsContainerEvent::TestMoved(p.step_a1, new_case),
        TestsContainerEvent::TestsModified
      ]
    );
  }

  #[gpui_kit::test]
  fn move_step_on_a_case_step_promotes_it_after(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    move_test(&tests, p.step_b1, p.case_step_a, cx).0.unwrap();
    assert_eq!(case_names(&tests, p.suite_a, cx), vec!["Case A", "Case step A", "Step B1"]);
    assert!(step_names(&tests, p.case_b, cx).is_empty());
  }

  #[gpui_kit::test]
  fn move_step_on_a_multi_case_appends_it(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let (result, events) = move_test(&tests, p.step_a1, p.case_b, cx);
    result.unwrap();
    let new_step = TestPath::Step(p.suite_b.id(), p.case_b.id(), p.step_a1.id());
    assert_eq!(step_names(&tests, p.case_b, cx), vec!["Step B1", "Step A1"]);
    assert_eq!(
      events,
      vec![
        TestsContainerEvent::TestMoved(p.step_a1, new_step),
        TestsContainerEvent::TestsModified
      ]
    );
  }

  #[gpui_kit::test]
  fn move_step_on_a_step_places_it_after(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let (result, events) = move_test(&tests, p.step_a2, p.step_a1, cx);
    result.unwrap();
    // Already right after the target
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A1", "Step A2"]);
    assert_eq!(events, vec![TestsContainerEvent::TestsModified]);

    move_test(&tests, p.step_a1, p.step_a2, cx).0.unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A2", "Step A1"]);

    move_test(&tests, p.step_b1, p.step_a2, cx).0.unwrap();
    assert_eq!(step_names(&tests, p.case_a, cx), vec!["Step A2", "Step B1", "Step A1"]);
  }

  #[gpui_kit::test]
  fn move_test_keeps_the_moved_entity(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    let case_a = case(&tests, p.case_a, cx);
    let step_b1 = step(&tests, p.step_b1, cx);
    move_test(&tests, p.case_a, p.suite_b, cx).0.unwrap();
    let moved_step_a1 = TestPath::Step(p.suite_b.id(), p.case_a.id(), p.step_a1.id());
    move_test(&tests, p.step_b1, moved_step_a1, cx).0.unwrap();
    assert_eq!(case(&tests, TestPath::Case(p.suite_b.id(), p.case_a.id()), cx), case_a);
    let moved_step = TestPath::Step(p.suite_b.id(), p.case_a.id(), p.step_b1.id());
    assert_eq!(step(&tests, moved_step, cx), step_b1);
    assert_eq!(step_b1.read_with(cx, |step, _| step.path()), moved_step);
  }

  #[gpui_kit::test]
  fn saved_tree_follows_moves(cx: &mut TestAppContext) {
    let (p, tests) = fixture(cx);
    move_test(&tests, p.case_step_a, p.case_b, cx).0.unwrap();
    let file = cx.read(|cx| tests.read(cx).to_file(cx));
    assert_eq!(file.suites[0].cases.len(), 1);
    let FileTestCaseType::CaseMulti { steps } = &file.suites[1].cases[0].case_type else {
      panic!("Case B should be a multi-case");
    };
    assert_eq!(
      steps.iter().map(|step| step.info.id).collect::<Vec<_>>(),
      vec![p.step_b1.id(), p.case_step_a.id()]
    );
  }
}
