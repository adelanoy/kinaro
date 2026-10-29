use crate::app;
use crate::ui::components::tree::{ProjectTreeNode, Tree, TreeDelegate, TreeEvent, TreeState};
use gpui_kit::base::actions::Cancel;
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
use ki_app_shared::actions::{
  AddDelayStep, AddRestStep, AddTestCase, AddTestSuite, CopyId, CopyPath, Delete, Duplicate, Edit, MoveDown, MoveUp,
  PROJECT_TREE_CONTEXT_KEY, SwitchNodeActiveStatus,
};
use ki_app_shared::dialog::show_rename_dialog;
use ki_app_shared::panel::PanelDescriptor;
use ki_app_shared::{ContextMenuBuilder, MovingLabel};
use ki_assets::icon::IconAsset;
use ki_utils::http::HttpMethod;
use ki_utils::{CASE, DropPosition, STEP, SUITE, SidebarPanel};
use ki_utils::{Offset, TestPath};
use ki_workspace::error::ProjectError;
use ki_workspace::test::{TestMetadataOwner, TestsContainer, can_move};
use ki_workspace::{Project, TestStep, TestSuite, TestSuiteChild};
use log::warn;
use std::collections::HashSet;

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

  /// Adds a new Rest step sending the action's method relative to the selected node, and selects it.
  fn on_action_add_rest_step(&mut self, action: &AddRestStep, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_action_add_rest_step: no tree_state selected path");
        return;
      };
      if let Some(new_path) = tree.delegate_mut().add_rest_step(path, action.method, window, cx) {
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
      ki_app_shared::copy_id(path, window, cx);
    });
  }

  /// Copies the path of the selected node to the clipboard.
  fn on_action_copy_path(&mut self, _action: &CopyPath, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_action_copy_path: no tree_state selected path");
        return;
      };
      ki_app_shared::copy_path(path, window, cx);
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
  fn on_action_escape(&mut self, _action: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
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
  fn on_action_rename(&mut self, _: &Edit, window: &mut Window, cx: &mut Context<Self>) {
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

impl PanelDescriptor<SidebarPanel, Project> for ProjectTree {
  fn name() -> &'static str {
    "Project Tree"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Tree
  }

  fn panel_type() -> SidebarPanel {
    SidebarPanel::ProjectTree
  }

  fn new(project: Entity<Project>, window: &mut Window, cx: &mut App) -> Entity<Self> {
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
  /// Where the dragged node would be dropped: the target node and the position relative to it.
  /// Only meaningful while a drag is active, as it isn't reset when a drag ends outside of the tree.
  drop_target: Option<(TestPath, DropPosition)>,
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

  /// Adds a new delay step relative to `path`: as a suite step when `path` is a suite or a suite
  /// step, as a step of the case otherwise. Returns the new step's path, or `None` after pushing a
  /// notification if that fails.
  fn add_delay_step(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Option<TestPath> {
    let new_path = match path {
      TestPath::Suite(_) | TestPath::SuiteStep(_, _) => self
        .tests
        .read(cx)
        .suite_from_path(&path, cx)
        .map(|suite| suite.update(cx, |suite, cx| suite.add_delay_step(path, cx))),
      TestPath::Case(_, _) | TestPath::Step(_, _, _) => self
        .tests
        .read(cx)
        .case_from_path(&path, cx)
        .map(|case| case.update(cx, |case, cx| case.add_delay_step(path, cx))),
    };
    if new_path.is_none() {
      window.push_notification(ProjectError::TestNotFound(path), cx);
    }
    new_path
  }

  /// Adds a new Rest step sending a `method` request relative to `path`: as a suite step when
  /// `path` is a suite or a suite step, as a step of the case otherwise. Returns the new step's
  /// path, or `None` after pushing a notification if that fails.
  fn add_rest_step(
    &mut self,
    path: TestPath,
    method: HttpMethod,
    window: &mut Window,
    cx: &mut Context<TreeState<Self>>,
  ) -> Option<TestPath> {
    let new_path = match path {
      TestPath::Suite(_) | TestPath::SuiteStep(_, _) => self
        .tests
        .read(cx)
        .suite_from_path(&path, cx)
        .map(|suite| suite.update(cx, |suite, cx| suite.add_rest_step(path, method, cx))),
      TestPath::Case(_, _) | TestPath::Step(_, _, _) => self
        .tests
        .read(cx)
        .case_from_path(&path, cx)
        .map(|case| case.update(cx, |case, cx| case.add_rest_step(path, method, cx))),
    };
    if new_path.is_none() {
      window.push_notification(ProjectError::TestNotFound(path), cx);
    }
    new_path
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
      TestPath::Case(_, _) | TestPath::SuiteStep(_, _) => {
        if let Some(suite) = self.tests.read(cx).suite_from_path(path, cx) {
          suite.update(cx, |suite, cx| suite.duplicate_child(path, cx))
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
      TestPath::Case(_, _) | TestPath::SuiteStep(_, _) => {
        if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
          suite.update(cx, |suite, cx| suite.offset_child(path, offset, cx))
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
  fn move_node(
    &mut self,
    from: TestPath,
    to: TestPath,
    position: DropPosition,
    window: &mut Window,
    cx: &mut Context<TreeState<Self>>,
  ) {
    match self.tests.update(cx, |tests, cx| tests.move_test(from, to, position, cx)) {
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
      drop_target: None,
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
  /// suite after a suite, a case in the node's suite, a step in the suite when pasted on a suite or
  /// a suite step, and in the case otherwise. Otherwise nothing happens.
  fn paste_node(&mut self, path: TestPath, cx: &mut Context<TreeState<Self>>) {
    if let Some(cb) = cx.read_from_clipboard()
      && let Some(ClipboardEntry::String(cb_str)) = cb.entries.first()
    {
      let metadata = cb_str.metadata.as_deref();
      match path {
        TestPath::Suite(_) if metadata == Some(SUITE) => {
          self
            .tests
            .update(cx, |tests, cx| tests.paste_from_clipboard(cb_str, path, cx));
        }
        _ if metadata == Some(CASE) => {
          if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
            suite.update(cx, |suite, cx| suite.paste_from_clipboard(cb_str, path, cx));
          }
        }
        TestPath::Suite(_) | TestPath::SuiteStep(_, _) if metadata == Some(STEP) => {
          if let Some(suite) = self.tests.read(cx).suite_from_path(&path, cx) {
            suite.update(cx, |suite, cx| suite.paste_from_clipboard(cb_str, path, cx));
          }
        }
        TestPath::Case(_, _) | TestPath::Step(_, _, _) if metadata == Some(STEP) => {
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
      TestPath::SuiteStep(_, _) | TestPath::Step(_, _, _) => {
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
          case.update(cx, |case, cx| case.switch_disabled(cx));
        }
      }
      TestPath::SuiteStep(_, _) | TestPath::Step(_, _, _) => {
        if let Some(step) = self.tests.read(cx).step_from_path(&path, cx) {
          step.update(cx, |step, cx| step.switch_disabled(cx));
        }
      }
    };
  }

  /// Rebuilds the rows from the tests.
  /// Returns where the node at `dragged` would be dropped when hovering the row at `ix`, `fraction`
  /// being the height of the cursor in the row (from 0 at the top to 1 at the bottom), or `None`
  /// when it can't be dropped there.
  ///
  /// Below an expanded node, the gap is the one above its first child: the node is placed before
  /// that child when it can be.
  fn drop_target_at(&self, ix: usize, dragged: TestPath, fraction: f32) -> Option<(TestPath, DropPosition)> {
    let node = self.tree_nodes.get(ix)?;
    let position = drop_position(dragged, node.path, fraction)?;
    if position == DropPosition::After
      && node.expanded
      && !node.leaf
      && let Some(first_child) = self.tree_nodes.get(ix + 1)
      && can_move(dragged, first_child.path, DropPosition::Before)
    {
      return Some((first_child.path, DropPosition::Before));
    }
    Some((node.path, position))
  }

  fn update_tests_nodes(&mut self, cx: &mut Context<TreeState<Self>>) {
    self.tree_nodes = self.tests.read_with(cx, |tests, _| {
      let opened_nodes = tests.opened_tree_nodes();
      get_ts_entries(&tests.suites, opened_nodes, cx)
    });
    cx.notify();
  }
}

/// Returns where the node at `dragged` would be dropped relative to the node at `target`, `fraction`
/// being the height of the cursor in the target's row (from 0 at the top to 1 at the bottom), or
/// `None` when it can't be dropped on `target`.
///
/// When the node can be dropped both into and next to `target`, the top and bottom quarters of the
/// row place it before and after `target`, the middle into it. When it can only be dropped next to
/// `target`, the top half places it before and the bottom half after.
fn drop_position(dragged: TestPath, target: TestPath, fraction: f32) -> Option<DropPosition> {
  let into = can_move(dragged, target, DropPosition::Into);
  // Before and After are allowed for the same nodes
  let sides = can_move(dragged, target, DropPosition::Before);
  match (into, sides) {
    (true, false) => Some(DropPosition::Into),
    (true, true) if fraction < 0.25 => Some(DropPosition::Before),
    (true, true) if fraction > 0.75 => Some(DropPosition::After),
    (true, true) => Some(DropPosition::Into),
    (false, true) if fraction < 0.5 => Some(DropPosition::Before),
    (false, true) => Some(DropPosition::After),
    (false, false) => None,
  }
}

/// Builds the rows of `suites` and, for the expanded ones, of their descendants.
fn get_ts_entries(suites: &[Entity<TestSuite>], opened_nodes: &HashSet<TestPath>, cx: &App) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  let last_ix = if suites.is_empty() { 0 } else { suites.len() - 1 };
  for (ix, suite) in suites.iter().enumerate() {
    suite.read_with(cx, |suite, cx| {
      let leaf = suite.children.is_empty();
      let path = suite.meta.path();
      let disabled = suite.meta.disabled;
      let expanded = opened_nodes.contains(&path);
      entries.push(ProjectTreeNode {
        path,
        label: suite.name(),
        icon: suite.icon(cx),
        disabled,
        parent_disabled: false,
        depth: 0,
        leaf,
        expanded,
        first: ix == 0,
        last: ix == last_ix,
      });
      if !leaf && expanded {
        entries.extend(get_child_entries(&suite.children, disabled, opened_nodes, cx));
      }
    });
  }
  entries
}

/// Builds the rows of a suite's `children` and, for the expanded cases, of their steps.
/// `parent_disabled` tells whether their suite is disabled.
fn get_child_entries(
  children: &[TestSuiteChild],
  parent_disabled: bool,
  opened_nodes: &HashSet<TestPath>,
  cx: &App,
) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  let last_ix = children.len() - 1;
  for (ix, child) in children.iter().enumerate() {
    match child {
      TestSuiteChild::Case(case) => case.read_with(cx, |case, cx| {
        let path = case.meta.path();
        let disabled = case.meta.disabled;
        let leaf = case.steps().is_empty();
        let expanded = opened_nodes.contains(&path);
        entries.push(ProjectTreeNode {
          path,
          label: case.name(),
          icon: case.icon(cx),
          disabled,
          parent_disabled,
          depth: 1,
          leaf,
          expanded,
          first: ix == 0,
          last: ix == last_ix,
        });
        if !leaf && expanded {
          entries.extend(get_step_entries(case.steps(), disabled || parent_disabled, cx));
        }
      }),
      TestSuiteChild::Step(step) => step.read_with(cx, |step, _| {
        entries.push(ProjectTreeNode {
          path: step.meta.path(),
          label: step.name(),
          icon: step.icon(cx),
          disabled: step.meta.disabled,
          parent_disabled,
          depth: 1,
          leaf: true,
          expanded: false,
          first: ix == 0,
          last: ix == last_ix,
        });
      }),
    }
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
        label: step.name(),
        icon: step.icon(cx),
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
  let path = node.path;
  let disabled = node.disabled;
  let first = node.first;
  let last = node.last;
  let builder = move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
    menu
      .submenu_with_icon(Some(IconName::Plus.into()), "New", window, cx, move |submenu, window, cx| {
        submenu
          .when(matches!(path, TestPath::Suite(_)), |submenu| {
            submenu.menu_with_icon("Test Suite", IconAsset::TestSuite, Box::new(AddTestSuite))
          })
          .when(matches!(path, TestPath::Suite(_) | TestPath::Case(_, _)), |submenu| {
            submenu.menu_with_icon("Test Case", IconAsset::TestCase, Box::new(AddTestCase))
          })
          .submenu_with_icon(
            Some(IconAsset::TestStep.into()),
            "Test Step",
            window,
            cx,
            move |submenu, _, _| {
              HttpMethod::ALL
              .iter()
              .fold(submenu, |submenu, method| {
                submenu.menu_with_icon(
                  format!("{} Request", method.as_str()),
                  method.icon(),
                  Box::new(AddRestStep { method: *method }),
                )
              })
              .separator()
              .menu_with_icon("Delay", IconAsset::TestStep, Box::new(AddDelayStep))
            },
          )
      })
      .separator()
      .menu_with_icon("Copy", IconName::Copy, Box::new(Copy))
      .menu("Copy id", Box::new(CopyId))
      .menu("Copy path", Box::new(CopyPath))
      .separator()
      .menu_with_icon("Duplicate", IconAsset::Duplicate, Box::new(Duplicate))
      .separator()
      .menu_with_check("Enabled", !disabled, Box::new(SwitchNodeActiveStatus))
      .menu_with_icon("Rename", IconAsset::Rename, Box::new(Edit))
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
    let icon = node.icon.clone();

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
        {
          let tree = cx.weak_entity();
          move |drag: &MovingLabel<TestPath>, _, _, cx| {
            cx.stop_propagation();
            _ = tree.update(cx, |tree, _| tree.delegate_mut().drop_target = None);
            cx.new(|_| drag.clone())
          }
        },
      )
      .on_drag_move(cx.listener(move |tree, event: &DragMoveEvent<MovingLabel<TestPath>>, _, cx| {
        // Called on every row during the drag: only the hovered one is the target
        let position = event.event.position;
        if !event.bounds.contains(&position) {
          return;
        }
        let fraction = (position.y - event.bounds.top()) / event.bounds.size.height;
        let dragged = event.drag(cx).data;
        let drop_target = tree.delegate().drop_target_at(ix, dragged, fraction);
        if tree.delegate().drop_target != drop_target {
          tree.delegate_mut().drop_target = drop_target;
          cx.notify();
        }
      }))
      .when(cx.has_active_drag(), |this| match self.drop_target {
        Some((target, DropPosition::Before)) if target == node.path => this.border_t_2().border_color(cx.theme().drop_target),
        Some((target, DropPosition::After)) if target == node.path => this.border_b_2().border_color(cx.theme().drop_target),
        Some((target, DropPosition::Into)) if target == node.path => this.rounded_md().bg(cx.theme().drop_target),
        _ => this,
      })
      .on_drop(cx.listener(move |tree, drag: &MovingLabel<TestPath>, window, cx| {
        if let Some((to, position)) = tree.delegate_mut().drop_target.take() {
          tree.delegate_mut().move_node(drag.data, to, position, window, cx);
        }
        cx.notify();
      }))
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
      .on_action(cx.listener(Self::on_action_add_rest_step))
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

#[cfg(test)]
mod tests {
  // Not a glob import of the parent module, whose `gpui_kit::*` import shadows `#[test]`
  use super::drop_position;
  use ki_utils::{DropPosition, TestPath};
  use uuid::Uuid;

  #[test]
  fn drop_position_on_a_suite() {
    let (suite, other_suite) = (TestPath::Suite(Uuid::new_v4()), TestPath::Suite(Uuid::new_v4()));
    // A suite goes before or after a suite
    assert_eq!(drop_position(other_suite, suite, 0.4), Some(DropPosition::Before));
    assert_eq!(drop_position(other_suite, suite, 0.6), Some(DropPosition::After));
    // Anything else goes into it, wherever the cursor is
    let case = TestPath::Case(Uuid::new_v4(), Uuid::new_v4());
    let step = TestPath::Step(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    for fraction in [0.1, 0.5, 0.9] {
      assert_eq!(drop_position(case, suite, fraction), Some(DropPosition::Into));
      assert_eq!(drop_position(step, suite, fraction), Some(DropPosition::Into));
    }
  }

  #[test]
  fn drop_position_on_a_case() {
    let suite_id = Uuid::new_v4();
    let case = TestPath::Case(suite_id, Uuid::new_v4());
    // A step goes before, into or after a case
    let step = TestPath::SuiteStep(suite_id, Uuid::new_v4());
    assert_eq!(drop_position(step, case, 0.2), Some(DropPosition::Before));
    assert_eq!(drop_position(step, case, 0.5), Some(DropPosition::Into));
    assert_eq!(drop_position(step, case, 0.8), Some(DropPosition::After));
    // A case only goes before or after it
    let other_case = TestPath::Case(suite_id, Uuid::new_v4());
    assert_eq!(drop_position(other_case, case, 0.4), Some(DropPosition::Before));
    assert_eq!(drop_position(other_case, case, 0.6), Some(DropPosition::After));
    // A suite can't be dropped on it
    assert_eq!(drop_position(TestPath::Suite(suite_id), case, 0.5), None);
  }

  #[test]
  fn drop_position_on_a_step() {
    let (suite_id, case_id) = (Uuid::new_v4(), Uuid::new_v4());
    let step = TestPath::Step(suite_id, case_id, Uuid::new_v4());
    let other_step = TestPath::Step(suite_id, case_id, Uuid::new_v4());
    assert_eq!(drop_position(other_step, step, 0.4), Some(DropPosition::Before));
    assert_eq!(drop_position(other_step, step, 0.6), Some(DropPosition::After));
    // A case can't be nested in a case
    assert_eq!(drop_position(TestPath::Case(suite_id, Uuid::new_v4()), step, 0.5), None);
    // Nor can a node be dropped on itself
    assert_eq!(drop_position(step, step, 0.5), None);
  }
}
