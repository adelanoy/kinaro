use crate::actions::{
  AddDelayStep, AddTestCase, AddTestSuite, CopyId, CopyPath, Delete, Duplicate, Escape, MoveDown, MoveUp,
  PROJECT_TREE_CONTEXT_KEY, Rename, SwitchNodeActiveStatus,
};
use crate::app;
use crate::ui::ProjectTestNodeKind;
use crate::ui::components::dialog::show_rename_dialog;
use crate::ui::components::tree::{ProjectTreeNode, Tree, TreeDelegate, TreeEvent, TreeState};
use crate::ui::side_bar::ProjectConfigurationPanel;
use gpui_kit::base::input::{Copy, Paste};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{
  ActiveTheme, Disableable, Icon, IconName, Sizable, WindowExt,
  button::{Button, ButtonVariants},
  h_flex,
  menu::{ContextMenuExt, PopupMenu},
  v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use ki_assets::icon::IconAsset;
use ki_utils::ui::MovingLabel;
use ki_utils::{CASE_MULTI, CASE_STEP, STEP, SUITE, SidebarPanel};
use ki_utils::{Offset, TestPath};
use ki_workspace::error::ProjectError;
use ki_workspace::test::test_case::TestCaseType;
use ki_workspace::test::{TestMetadataOwner, TestsContainer};
use ki_workspace::{Project, TestCase, TestStep, TestSuite};
use log::warn;
use std::collections::HashSet;

/// Builds the context menu of a tree node.
type ContextMenuBuilder = dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu;

/// The project tree panel: shows the project's test suites, cases and steps as a tree, and edits
/// them through keyboard actions, a context menu and drag and drop.
pub(in crate::ui::side_bar) struct ProjectTree {
  /// State of the displayed tree
  tree_state: Entity<TreeState<ProjectTreeDelegate>>,
  _tree_sub: Subscription,
}

impl ProjectTree {
  /// Adds a new test case relative to the selected node, and selects it.
  fn on_action_add_test_case(&mut self, _: &AddTestCase, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_add_test_case: no tree_state selected path");
        return;
      };
      if let Some(new_path) = tree.delegate_mut().add_test_case(path, window, cx) {
        tree.set_selected_path(Some(new_path));
      }
    });
  }

  /// Adds a new delay step relative to the selected node, and selects it.
  fn on_action_add_delay_step(&mut self, _: &AddDelayStep, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_action_add_delay_step: no tree_state selected path");
        return;
      };
      if let Some(new_path) = tree.delegate_mut().add_delay_step(path, window, cx) {
        tree.set_selected_path(Some(new_path));
      }
    });
  }

  /// Adds a new test suite after the selected node's suite (or at the end), and selects it.
  fn on_action_add_test_suite(&mut self, _: &AddTestSuite, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let path = tree.selected_path();
      if let Some(new_path) = tree.delegate_mut().add_test_suite(path, window, cx) {
        tree.set_selected_path(Some(new_path));
      }
    });
  }

  /// Copies the selected node to the clipboard.
  fn on_action_copy(&mut self, _action: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_action_copy: no tree_state selected path");
        return;
      };
      tree.delegate_mut().copy_node(path, cx);
    });
  }

  /// Copies the id of the selected node to the clipboard.
  fn on_action_copy_id(&mut self, _action: &CopyId, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_action_copy_id: no tree_state selected path");
        return;
      };
      app::copy_id(path, window, cx);
    });
  }

  /// Copies the path of the selected node to the clipboard.
  fn on_action_copy_path(&mut self, _action: &CopyPath, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_action_copy_path: no tree_state selected path");
        return;
      };
      app::copy_path(path, window, cx);
    });
  }

  /// Asks for confirmation before deleting the selected node.
  fn on_action_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_delete_node: no tree_state selected path");
        return;
      };
      tree.delegate_mut().show_delete_confirm_dialog(path, window, cx);
    });
  }

  /// Duplicates the selected node.
  fn on_action_duplicate(&mut self, _: &Duplicate, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_duplicate_node: no tree_state selected path");
        return;
      };
      tree.delegate_mut().duplicate_node(&path, window, cx);
    });
  }

  /// Clears the selection.
  fn on_action_escape(&mut self, _action: &Escape, _window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree_state, cx| tree_state.clear_selection(cx));
  }

  /// Moves the selected node one position up among its siblings.
  fn on_action_move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_move_up_action: no tree_state selected path");
        return;
      };
      tree.delegate_mut().offset_node(path, Offset::Minus, window, cx);
    });
  }

  /// Moves the selected node one position down among its siblings.
  fn on_action_move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_move_down_action: no tree_state selected path");
        return;
      };
      tree.delegate_mut().offset_node(path, Offset::Plus, window, cx);
    });
  }

  /// Opens the rename dialog of the selected node.
  fn on_action_rename(&mut self, _: &Rename, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_rename_node: no tree_state selected path");
        return;
      };
      tree.delegate_mut().show_rename_dialog(path, window, cx);
    });
  }

  /// Pastes the clipboard's content relative to the selected node.
  fn on_action_paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_rename_node: no tree_state selected path");
        return;
      };
      tree.delegate_mut().paste_node(path, cx);
    });
  }

  /// Enables the selected node if it is disabled, disables it otherwise.
  fn on_action_switch_node_active_status(
    &mut self,
    _action: &SwitchNodeActiveStatus,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_switch_node_enable_status: no tree_state selected path");
        return;
      };
      tree.delegate_mut().switch_node_enable_status(path, cx);
    })
  }

  /// Saves the expanded state of tree nodes, and opens double-clicked nodes in the editor.
  fn on_tree_event(
    &mut self,
    tree: &Entity<TreeState<ProjectTreeDelegate>>,
    e: &TreeEvent,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    tree.update(cx, |tree, cx| match e {
      TreeEvent::NodeExpanded(path) => tree.delegate_mut().on_expand_status_change(*path, true, cx),
      TreeEvent::NodeCollapsed(path) => tree.delegate_mut().on_expand_status_change(*path, false, cx),
      TreeEvent::EntryDoubleClicked(path) => tree.delegate_mut().on_node_double_clicked(*path, cx),
    });
  }
}

impl ProjectConfigurationPanel for ProjectTree {
  fn name() -> &'static str {
    "Project Tree"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Tree
  }

  fn panel_type() -> SidebarPanel {
    SidebarPanel::ProjectTree
  }

  fn new(project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<impl Render> {
    cx.new(|cx| {
      let tests = project.read(cx).tests.clone();
      let tree_state = cx.new(|cx| TreeState::new(ProjectTreeDelegate::new(tests, window, cx), cx));
      let _tree_sub = cx.subscribe_in(&tree_state, window, Self::on_tree_event);

      Self { tree_state, _tree_sub }
    })
  }
}

/// Drives the project tree from the project's tests, rebuilding its rows whenever they change.
struct ProjectTreeDelegate {
  /// The project's test tree
  tests: Entity<TestsContainer>,
  /// The visible rows (expanded nodes' children only), in display order
  tree_nodes: Vec<ProjectTreeNode>,
  _tests_sub: Subscription,
}

impl ProjectTreeDelegate {
  /// Adds a new test case relative to `path` in its suite. Returns the new case's path, or `None`
  /// after pushing a notification if `path`'s suite doesn't exist.
  fn add_test_case(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Option<TestPath> {
    match self
      .tests
      .read(cx)
      .suite_from_path(&path, cx)
      .map(|suite| suite.update(cx, |suite, cx| suite.add_new_test_case(path, cx)))
      .ok_or(ProjectError::TestNotFound(path))
    {
      Ok(path) => Some(path),
      Err(err) => {
        window.push_notification(err, cx);
        None
      }
    }
  }

  /// Adds a new delay step relative to `path`: as a case step of the suite when `path` is a suite
  /// or a case step, as a step of the case otherwise. Returns the new step's path, or `None` after
  /// pushing a notification if that fails.
  fn add_delay_step(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Option<TestPath> {
    if let Some(node) = self.tree_nodes.iter().find(|node| node.path == path) {
      match match node.kind {
        ProjectTestNodeKind::Suite | ProjectTestNodeKind::CaseStep => self
          .tests
          .read(cx)
          .suite_from_path(&path, cx)
          .map(|suite| suite.update(cx, |suite, cx| suite.add_new_delay_step(path, cx)))
          .ok_or(ProjectError::TestNotFound(path)),
        _ => {
          if let Some(case) = self.tests.read(cx).case_from_path(&path, cx) {
            case.update(cx, |case, cx| case.add_delay_step(path, cx))
          } else {
            Err(ProjectError::TestNotFound(path))
          }
        }
      } {
        Ok(path) => Some(path),
        Err(err) => {
          window.push_notification(err, cx);
          None
        }
      }
    } else {
      window.push_notification(ProjectError::TestNotFound(path), cx);
      None
    }
  }

  /// Adds a new test suite after the suite `path` goes through, or at the end. Returns its path.
  fn add_test_suite(
    &mut self,
    path: Option<TestPath>,
    _window: &mut Window,
    cx: &mut Context<TreeState<Self>>,
  ) -> Option<TestPath> {
    Some(self.tests.update(cx, |tests, cx| tests.add_test_suite(path, cx)))
  }

  /// Copies the node at `path` to the clipboard.
  fn copy_node(&mut self, path: TestPath, cx: &mut Context<TreeState<Self>>) {
    app::copy_test(path, &self.tests, cx);
  }

  /// Deletes the node at `path` and its descendants.
  fn delete_node(&mut self, path: &TestPath, _window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    self.tests.update(cx, |tests, cx| tests.remove_test(path, cx));
  }

  /// Duplicates the node at `path`. Returns the path of the copy, or `None` after pushing a
  /// notification if that fails.
  fn duplicate_node(&mut self, path: &TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Option<TestPath> {
    match match path {
      TestPath::Suite(_) => self.tests.update(cx, |tests, cx| tests.duplicate_suite(path, cx)),
      TestPath::Case(_, _) => {
        if let Some(suite) = self.tests.read(cx).suite_from_path(path, cx) {
          suite.update(cx, |suite, cx| suite.duplicate_case(path, cx))
        } else {
          Err(ProjectError::TestNotFound(*path))
        }
      }
      TestPath::Step(_, _, _) => {
        if let Some(case) = self.tests.read(cx).case_from_path(path, cx) {
          case.update(cx, |case, cx| case.duplicate_step(path, cx))
        } else {
          Err(ProjectError::TestNotFound(*path))
        }
      }
    } {
      Ok(path) => Some(path),
      Err(err) => {
        window.push_notification(err, cx);
        None
      }
    }
  }

  /// Moves the node at `path` one position among its siblings, pushing a notification if that fails.
  fn offset_node(&mut self, path: TestPath, offset: Offset, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    if let Err(err) = match path {
      TestPath::Suite(_) => self.tests.update(cx, |tests, cx| tests.offset_suite(path, offset, cx)),
      TestPath::Case(_, _) => {
        if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
          suite.update(cx, |suite, cx| suite.offset_case(path, offset, cx))
        } else {
          Err(ProjectError::TestNotFound(path))
        }
      }
      TestPath::Step(_, _, _) => {
        if let Some(case) = self.tests.read(cx).case_from_path(&path, cx) {
          case.update(cx, |case, cx| case.offset_step(path, offset, cx))
        } else {
          Err(ProjectError::TestNotFound(path))
        }
      }
    } {
      window.push_notification(err, cx);
    }
  }

  /// Moves the node `from` relative to the node `to` (drag and drop), pushing a notification if that
  /// fails, unless the move is just not allowed.
  fn move_node(&mut self, from: TestPath, to: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    match self.tests.update(cx, |tests, cx| tests.move_test(from, to, cx)) {
      Ok(_) | Err(ProjectError::NoOp) => {}
      Err(err) => window.push_notification(err, cx),
    }
  }

  /// Creates the delegate of `tests`, building the rows from its current state.
  fn new(tests: Entity<TestsContainer>, _window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Self {
    let _tests_sub = cx.subscribe(&tests, |this, _, _, cx| {
      this.delegate_mut().update_tests_nodes(cx);
    });
    let mut delegate = Self {
      tests,
      tree_nodes: vec![],
      _tests_sub,
    };
    delegate.update_tests_nodes(cx);
    delegate
  }

  /// Saves whether the node at `path` is expanded (`open`) or collapsed.
  fn on_expand_status_change(&mut self, path: TestPath, open: bool, cx: &mut Context<TreeState<Self>>) {
    self.tests.update(cx, move |tests, cx| {
      if open {
        tests.expand_tree_node(path, cx);
      } else {
        tests.collapse_tree_node(&path, cx);
      }
    });
  }

  /// Requests the node at `path` to be opened in the editor.
  fn on_node_double_clicked(&mut self, path: TestPath, cx: &mut Context<TreeState<Self>>) {
    self.tests.update(cx, move |tests, cx| {
      tests.request_open_in_editor(path, cx);
    });
  }

  /// Pastes the clipboard's content relative to the node at `path`, when its kind can go there: a
  /// suite after a suite, a multi-step case in the node's suite, a step or a case step in the suite
  /// when pasted on a suite or a case step, and in the case otherwise. Otherwise nothing happens.
  fn paste_node(&mut self, path: TestPath, cx: &mut Context<TreeState<Self>>) {
    if let Some(cb) = cx.read_from_clipboard()
      && let Some(ClipboardEntry::String(cb_str)) = cb.entries.first()
    {
      let Some(kind) = self.tree_nodes.iter().find(|node| node.path == path).map(|node| node.kind) else {
        warn!("ProjectTreeDelegate:paste_node: unknown path: {}", path);
        return;
      };
      let metadata = cb_str.metadata.as_deref();
      let is_step = metadata == Some(STEP) || metadata == Some(CASE_STEP);
      match kind {
        ProjectTestNodeKind::Suite if metadata == Some(SUITE) => {
          self
            .tests
            .update(cx, |tests, cx| tests.paste_from_clipboard(cb_str, path, cx));
        }
        _ if metadata == Some(CASE_MULTI) => {
          if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
            suite.update(cx, |suite, cx| suite.paste_from_clipboard(cb_str, path, cx));
          }
        }
        ProjectTestNodeKind::Suite | ProjectTestNodeKind::CaseStep if is_step => {
          if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
            suite.update(cx, |suite, cx| suite.paste_from_clipboard(cb_str, path, cx));
          }
        }
        ProjectTestNodeKind::Case | ProjectTestNodeKind::Step if is_step => {
          if let Some(case) = self.tests.read(cx).case_from_path(&path, cx) {
            case.update(cx, |case, cx| case.paste_from_clipboard(cb_str, path, cx));
          }
        }
        _ => {}
      };
    }
  }

  /// Opens the rename dialog of the node at `path`, if it is in the tree.
  fn show_rename_dialog(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(node) = self.tree_nodes.iter().find(|node| node.path == path) else {
      warn!("ProjectTreeDelegate:setup_rename_node: unknown path: {}", path);
      return;
    };
    let name = node.label.clone();
    match path {
      TestPath::Suite(_) => {
        if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
          show_rename_dialog(name, suite, window, cx)
        }
      }
      TestPath::Case(_, _) => {
        if let Some(case) = self.tests.read(cx).case_from_path(&path, cx) {
          show_rename_dialog(name, case, window, cx)
        }
      }
      TestPath::Step(_, _, _) => {
        if let Some(step) = self.tests.read(cx).step_from_path(&path, cx) {
          show_rename_dialog(name, step, window, cx)
        }
      }
    };
  }

  /// Opens a dialog asking for confirmation before deleting the node at `path`, if it is in the tree.
  fn show_delete_confirm_dialog(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(node) = self.tree_nodes.iter().find(|node| node.path == path) else {
      warn!("ProjectTreeDelegate:delete_node: unknown path: {}", path);
      return;
    };

    let this = cx.entity();
    window.open_alert_dialog(cx, {
      let kind = node.label();
      let label = node.label.clone();
      let path = node.path;
      move |dialog, _, _| {
        dialog
          .title(format!("Delete {}", kind))
          .description(format!("You are about to delete the {} '{}', continue?", kind, label))
          .show_cancel(true)
          .on_ok({
            let this = this.clone();
            move |_, window, cx| {
              this.update(cx, |this, cx| {
                this.delegate_mut().delete_node(&path, window, cx);
                this.clear_selection(cx);
              });
              true
            }
          })
      }
    });
  }

  /// Enables the node at `path` if it is disabled, disables it otherwise.
  fn switch_node_enable_status(&mut self, path: TestPath, cx: &mut Context<TreeState<Self>>) {
    match path {
      TestPath::Suite(_) => {
        if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
          suite.update(cx, |suite, cx| suite.switch_disabled(cx));
        }
      }
      TestPath::Case(_, _) => {
        if let Some(case) = self.tests.read(cx).case_from_path(&path, cx) {
          case.update(cx, |suite, cx| suite.switch_disabled(cx));
        }
      }
      TestPath::Step(_, _, _) => {
        if let Some(step) = self.tests.read(cx).step_from_path(&path, cx) {
          step.update(cx, |suite, cx| suite.switch_disabled(cx));
        }
      }
    };
  }

  /// Rebuilds the rows from the tests.
  fn update_tests_nodes(&mut self, cx: &mut Context<TreeState<Self>>) {
    self.tree_nodes = self.tests.read_with(cx, |tests, _| {
      let opened_nodes = tests.opened_tree_nodes();
      get_ts_entries(&tests.suites, opened_nodes, cx)
    });
    cx.notify();
  }
}

/// Builds the rows of `suites` and, for the expanded ones, of their descendants.
fn get_ts_entries(suites: &[Entity<TestSuite>], opened_nodes: &HashSet<TestPath>, cx: &App) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  let last_ix = if suites.is_empty() { 0 } else { suites.len() - 1 };
  for (ix, suite) in suites.iter().enumerate() {
    suite.read_with(cx, |suite, cx| {
      let leaf = suite.cases.is_empty();
      let path = suite.meta.path();
      let disabled = suite.meta.disabled;
      let expanded = opened_nodes.contains(&path);
      entries.push(ProjectTreeNode {
        path,
        kind: ProjectTestNodeKind::Suite,
        label: suite.name(),
        disabled,
        parent_disabled: false,
        depth: 0,
        leaf,
        expanded,
        first: ix == 0,
        last: ix == last_ix,
      });
      if !leaf && expanded {
        entries.extend(get_tc_entries(&suite.cases, disabled, opened_nodes, cx));
      }
    });
  }
  entries
}

/// Builds the rows of `cases` and, for the expanded ones, of their steps. `parent_disabled` tells
/// whether their suite is disabled.
fn get_tc_entries(
  cases: &[Entity<TestCase>],
  parent_disabled: bool,
  opened_nodes: &HashSet<TestPath>,
  cx: &App,
) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  let last_ix = cases.len() - 1;
  for (ix, case) in cases.iter().enumerate() {
    case.read_with(cx, |case, _| {
      let path = case.meta.path();
      let disabled = case.meta.disabled;
      let mut case_tree_node = ProjectTreeNode {
        path,
        kind: ProjectTestNodeKind::Case,
        label: case.name(),
        disabled,
        parent_disabled,
        depth: 1,
        leaf: true,
        expanded: false,
        first: ix == 0,
        last: ix == last_ix,
      };
      match case.case_type() {
        TestCaseType::CaseMulti { steps } => {
          let is_empty = steps.is_empty();
          let is_open = opened_nodes.contains(&path);
          case_tree_node.leaf = is_empty;
          case_tree_node.expanded = is_open;
          entries.push(case_tree_node);
          if !is_empty && is_open {
            entries.extend(get_step_entries(steps, disabled || parent_disabled, cx));
          }
        }
        TestCaseType::CaseStep { .. } => {
          case_tree_node.kind = ProjectTestNodeKind::CaseStep;
          entries.push(case_tree_node);
        }
      }
    });
  }
  entries
}

/// Builds the rows of `steps`. `parent_disabled` tells whether their case or suite is disabled.
fn get_step_entries(steps: &[Entity<TestStep>], parent_disabled: bool, cx: &App) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  let last_ix = steps.len() - 1;
  for (ix, step) in steps.iter().enumerate() {
    step.read_with(cx, |step, _| {
      let disabled = step.meta.disabled;
      entries.push(ProjectTreeNode {
        path: step.meta.path(),
        kind: ProjectTestNodeKind::Step,
        label: step.name(),
        disabled,
        parent_disabled,
        depth: 2,
        leaf: true,
        expanded: false,
        first: ix == 0,
        last: ix == last_ix,
      });
    });
  }
  entries
}

/// Returns the builder of the context menu of `node`, offering the actions allowed on its kind.
fn build_context_menu(node: &ProjectTreeNode) -> Box<ContextMenuBuilder> {
  let kind = node.kind;
  let disabled = node.disabled;
  let first = node.first;
  let last = node.last;
  let builder = move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
    menu
      .submenu_with_icon(Some(IconName::Plus.into()), "New", window, cx, move |submenu, _, _| {
        let mut submenu = submenu;
        if matches!(kind, ProjectTestNodeKind::Suite) {
          submenu = submenu.menu_with_icon("Test Suite", IconAsset::TestSuite, Box::new(AddTestSuite));
        }
        if matches!(kind, ProjectTestNodeKind::Suite | ProjectTestNodeKind::Case) {
          submenu = submenu.menu_with_icon("Test Case", IconAsset::TestCase, Box::new(AddTestCase));
        }
        submenu.menu_with_icon("Test Step", IconAsset::TestStep, Box::new(AddDelayStep))
      })
      .separator()
      .menu_with_icon("Copy", IconName::Copy, Box::new(Copy))
      .menu("Copy id", Box::new(CopyId))
      .menu("Copy path", Box::new(CopyPath))
      .separator()
      .menu_with_icon("Duplicate", IconAsset::Duplicate, Box::new(Duplicate))
      .separator()
      .menu_with_check("Enabled", !disabled, Box::new(SwitchNodeActiveStatus))
      .menu_with_icon("Rename", IconAsset::Rename, Box::new(Rename))
      .separator()
      .menu_with_icon("Delete", IconName::Delete, Box::new(Delete))
      .separator()
      .menu_with_icon_and_disabled("Move Up", IconName::ArrowUp, Box::new(MoveUp), first)
      .menu_with_icon_and_disabled("Move Down", IconName::ArrowDown, Box::new(MoveDown), last)
  };
  Box::new(builder)
}

impl TreeDelegate for ProjectTreeDelegate {
  fn find_above(&self, path: &TestPath) -> Option<TestPath> {
    let pos = self.tree_nodes.iter().position(|node| node.path == *path)?;
    if pos > 0 { Some(self.tree_nodes[pos - 1].path) } else { None }
  }

  fn find_below(&self, path: &TestPath) -> Option<TestPath> {
    let pos = self.tree_nodes.iter().position(|node| node.path == *path)?;
    if pos < self.tree_nodes.len() - 1 {
      Some(self.tree_nodes[pos + 1].path)
    } else {
      None
    }
  }

  fn row_count(&self) -> usize {
    self.tree_nodes.len()
  }

  fn node(&self, ix: usize, _cx: &Context<TreeState<Self>>) -> &ProjectTreeNode {
    self.tree_nodes.get(ix).unwrap()
  }

  fn node_render(&self, ix: usize, _selected: bool, _window: &mut Window, cx: &mut Context<TreeState<Self>>) -> impl IntoElement {
    let node = &self.tree_nodes[ix];
    let id = node.path.id();
    let icon = node.kind.icon();

    h_flex()
      .id(node.id())
      .size_full()
      .gap_x_1()
      .child(icon)
      .tooltip(move |window, cx| Tooltip::new(format!("{}", id.clone())).build(window, cx))
      .child(node.label.clone())
      .when(node.disabled, |this| this.child(Icon::new(IconAsset::Ban).xsmall()))
      .when(node.disabled || node.parent_disabled, |this| {
        this.text_color(cx.theme().muted_foreground)
      })
      .on_drag(
        MovingLabel {
          label: node.label.clone(),
          data: node.path,
        },
        |drag: &MovingLabel<TestPath>, _, _, cx| {
          cx.stop_propagation();
          cx.new(|_| drag.clone())
        },
      )
      .on_drop({
        let to_path = node.path;
        cx.listener(move |table, e: &MovingLabel<TestPath>, window, cx| {
          table.delegate_mut().move_node(e.data, to_path, window, cx);
        })
      })
      .context_menu(build_context_menu(node))
  }
}

impl Render for ProjectTree {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let selected_path = self.tree_state.read(cx).selected_path();

    v_flex()
      .id("project-tree")
      .key_context(PROJECT_TREE_CONTEXT_KEY)
      .on_action(cx.listener(Self::on_action_add_test_case))
      .on_action(cx.listener(Self::on_action_add_delay_step))
      .on_action(cx.listener(Self::on_action_add_test_suite))
      .on_action(cx.listener(Self::on_action_copy))
      .on_action(cx.listener(Self::on_action_copy_id))
      .on_action(cx.listener(Self::on_action_copy_path))
      .on_action(cx.listener(Self::on_action_delete))
      .on_action(cx.listener(Self::on_action_duplicate))
      .on_action(cx.listener(Self::on_action_escape))
      .on_action(cx.listener(Self::on_action_move_up))
      .on_action(cx.listener(Self::on_action_move_down))
      .on_action(cx.listener(Self::on_action_paste))
      .on_action(cx.listener(Self::on_action_rename))
      .on_action(cx.listener(Self::on_action_switch_node_active_status))
      .size_full()
      .gap_y_2()
      .p_1()
      .child(
        h_flex().gap_x_2().w_full().flex_row_reverse().child(
          Button::new("btn-remove-tree-node")
            .ghost()
            .small()
            .disabled(selected_path.is_none())
            .icon(IconName::Delete),
        ),
      )
      .child(Tree::new(self.tree_state.clone()))
  }
}
