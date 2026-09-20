use crate::actions::{
  AddTestCase, AddTestStep, AddTestSuite, Delete, Duplicate, Escape, MoveDown, MoveUp, PROJECT_TREE_CONTEXT_KEY, Rename,
  SwitchNodeActiveStatus,
};
use crate::ui::components::tree::{ProjectTreeNode, ProjectTreeNodeKind, Tree, TreeDelegate, TreeEvent, TreeState};
use crate::ui::side_bar::project_configuration::ProjectConfigurationTab;
use gpui_kit::component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_kit::component::input::{Input, InputState};
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
use ki_utils::Offset;
use ki_utils::ui::MovingLabel;
use ki_workspace::test::test_case::TestCaseType;
use ki_workspace::test::{TestPath, TestsContainer};
use ki_workspace::{Project, TestCase, TestSuite};
use log::warn;
use std::collections::HashSet;
use uuid::Uuid;

type ContextMenuBuilder = dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu;

pub(super) struct ProjectTree {
  tree_state: Entity<TreeState<ProjectTreeDelegate>>,
  _tree_sub: Subscription,
}

impl ProjectTree {
  fn on_tree_event(
    &mut self,
    tree: &Entity<TreeState<ProjectTreeDelegate>>,
    e: &TreeEvent,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    tree.update(cx, |tree, cx| match e {
      TreeEvent::NodeExpanded(path) => tree.delegate_mut().on_expand_status_change(path, true, cx),
      TreeEvent::NodeCollapsed(path) => tree.delegate_mut().on_expand_status_change(path, false, cx),
      TreeEvent::EntryDoubleClicked(ix) => println!("Entry action: {}", ix),
    });
  }

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
      tree.delegate_mut().on_switch_node_enable_status(path, cx);
    })
  }

  fn on_action_rename(&mut self, _: &Rename, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_rename_node: no tree_state selected path");
        return;
      };
      tree.delegate_mut().show_rename_dialog(path, window, cx);
    });
  }

  fn on_action_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_delete_node: no tree_state selected path");
        return;
      };
      tree.delegate_mut().show_delete_confirm_dialog(path, window, cx);
    });
  }

  fn on_action_duplicate(&mut self, _: &Duplicate, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_duplicate_node: no tree_state selected path");
        return;
      };
      tree.delegate_mut().duplicate_node(path, window, cx);
    });
  }

  fn on_action_add_test_suite(&mut self, _: &AddTestSuite, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let path = tree.selected_path();
      if let Some(new_path) = tree.delegate_mut().add_test_suite(path, window, cx) {
        tree.set_selected_path(Some(new_path));
      }
    });
  }

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

  fn on_action_add_test_step(&mut self, _: &AddTestStep, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_add_step: no tree_state selected path");
        return;
      };
      if let Some(new_path) = tree.delegate_mut().add_test_step(path, window, cx) {
        tree.set_selected_path(Some(new_path));
      }
    });
  }

  fn on_action_move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_move_up_action: no tree_state selected path");
        return;
      };
      tree.delegate_mut().move_up(path, window, cx);
    });
  }

  fn on_action_move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(path) = tree.selected_path() else {
        warn!("ProjectTree:on_move_down_action: no tree_state selected path");
        return;
      };
      tree.delegate_mut().move_down(path, window, cx);
    });
  }

  fn on_action_escape(&mut self, _action: &Escape, _window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree_state, cx| tree_state.clear_selection(cx));
  }
}

impl ProjectConfigurationTab for ProjectTree {
  fn name() -> &'static str {
    "Project Tree"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Tree
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

struct ProjectTreeDelegate {
  tests: Entity<TestsContainer>,
  tree_nodes: Vec<ProjectTreeNode>,
  _tests_sub: Subscription,
}

impl ProjectTreeDelegate {
  fn add_test_case(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Option<TestPath> {
    match self.tests.update(cx, |tests, cx| tests.add_test_case(path, cx)) {
      Ok(path) => Some(path),
      Err(err) => {
        window.push_notification(err, cx);
        None
      }
    }
  }

  fn add_test_from_path(
    &mut self,
    path: Option<TestPath>,
    window: &mut Window,
    cx: &mut Context<TreeState<Self>>,
  ) -> Option<TestPath> {
    match path {
      None => self.add_test_suite(None, window, cx),
      Some(path) => match self.tree_nodes.iter().find(|node| node.path == path) {
        None => {
          warn!("ProjectTreeDelegate:add_test_from_index: unknown path: {}", path);
          None
        }
        Some(node) => match node.node_kind {
          ProjectTreeNodeKind::Suite => self.add_test_suite(Some(path), window, cx),
          ProjectTreeNodeKind::Case => self.add_test_case(path, window, cx),
          ProjectTreeNodeKind::CaseStep | ProjectTreeNodeKind::Step => self.add_test_step(path, window, cx),
        },
      },
    }
  }

  fn add_test_step(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Option<TestPath> {
    match self.tests.update(cx, |tests, cx| tests.add_test_step(path, cx)) {
      Ok(path) => Some(path),
      Err(err) => {
        window.push_notification(err, cx);
        None
      }
    }
  }

  fn add_test_suite(
    &mut self,
    path: Option<TestPath>,
    _window: &mut Window,
    cx: &mut Context<TreeState<Self>>,
  ) -> Option<TestPath> {
    Some(self.tests.update(cx, |tests, cx| tests.add_test_suite(path, cx)))
  }

  fn delete_node(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.delete_test(&path, cx)) {
      window.push_notification(err, cx);
    }
  }

  fn duplicate_node(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.duplicate_test(&path, cx)) {
      window.push_notification(err, cx);
    }
  }

  fn move_node(&mut self, from: TestPath, to: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.move_test(from, to, cx)) {
      window.push_notification(err, cx);
    }
  }

  fn move_down(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.move_offset(path, Offset::Plus, cx)) {
      window.push_notification(err, cx);
    }
  }

  fn move_up(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.move_offset(path, Offset::Minus, cx)) {
      window.push_notification(err, cx);
    }
  }

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

  fn on_expand_status_change(&mut self, path: &TestPath, open: bool, cx: &mut Context<TreeState<Self>>) {
    let id = path.id();
    self.tests.update(cx, move |project, cx| {
      if open {
        project.expand_tree_node(id, cx);
      } else {
        project.collapse_tree_node(&id, cx);
      }
    });
  }

  fn on_switch_node_enable_status(&mut self, path: TestPath, cx: &mut Context<TreeState<Self>>) {
    self.tests.update(cx, move |tests, cx| {
      tests.switch_node_enable_status(&path, cx);
    });
  }

  fn show_rename_dialog(&mut self, path: TestPath, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(node) = self.tree_nodes.iter().find(|node| node.path == path) else {
      warn!("ProjectTreeDelegate:setup_rename_node: unknown path: {}", path);
      return;
    };
    let name = node.label.clone();
    let path = node.path;
    let input = cx.new(|cx| {
      let mut state = InputState::new(window, cx);
      state.set_value(name, window, cx);
      state
    });
    let tests = self.tests.clone();
    window.open_dialog(cx, move |dialog, window, cx| {
      input.update(cx, |input, cx| {
        input.focus(window, cx);
      });
      dialog
        .title("Rename test")
        .child(v_flex().gap_3().child("Enter the node's name:").child(Input::new(&input)))
        .footer(
          DialogFooter::new()
            .child(DialogClose::new().child(Button::new("cancel").label("Cancel").outline()))
            .child(DialogAction::new().child(Button::new("confirm").primary().label("Rename"))),
        )
        .on_ok({
          let name = input.clone().read(cx).value();
          let tests = tests.clone();
          move |_, window, cx| {
            if let Err(err) = tests.update(cx, |tests, cx| tests.rename_at(&path, name.clone(), cx)) {
              window.push_notification(err, cx);
            }
            true
          }
        })
    });
  }

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
                this.delegate_mut().delete_node(path, window, cx);
                this.clear_selection(cx);
              });
              true
            }
          })
      }
    });
  }

  fn update_tests_nodes(&mut self, cx: &mut Context<TreeState<Self>>) {
    self.tree_nodes = self.tests.read_with(cx, |tests, _| {
      let opened_nodes = tests.opened_tree_nodes();
      get_ts_entries(&tests.suites, opened_nodes)
    });
    cx.notify();
  }
}

fn get_ts_entries(suites: &[TestSuite], opened_nodes: &HashSet<Uuid>) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  let last_ix = suites.len() - 1;
  for (ix, suite) in suites.iter().enumerate() {
    let leaf = suite.cases.is_empty();
    let path = suite.meta.path();
    let disabled = suite.meta.disabled;
    let expanded = opened_nodes.contains(&path.id());
    entries.push(ProjectTreeNode {
      path,
      node_kind: ProjectTreeNodeKind::Suite,
      label: suite.meta.name.clone(),
      disabled,
      parent_disabled: false,
      depth: 0,
      leaf,
      expanded,
      first: ix == 0,
      last: ix == last_ix,
    });
    if !leaf && expanded {
      entries.extend(get_tc_entries(&suite.cases, disabled, opened_nodes));
    }
  }
  entries
}

fn get_tc_entries(cases: &[TestCase], parent_disabled: bool, opened_nodes: &HashSet<Uuid>) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  let last_ix = cases.len() - 1;
  for (ix, case) in cases.iter().enumerate() {
    let path = case.meta.path();
    let disabled = case.meta.disabled;
    let mut case_tree_node = ProjectTreeNode {
      path,
      node_kind: ProjectTreeNodeKind::Case,
      label: case.meta.name.clone(),
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
        let is_open = opened_nodes.contains(&path.id());
        case_tree_node.leaf = is_empty;
        case_tree_node.expanded = is_open;
        entries.push(case_tree_node);
        if !is_empty && is_open {
          let last_ix = steps.len() - 1;
          entries.extend(steps.iter().enumerate().map(|(ix, step)| ProjectTreeNode {
            path: step.meta.path(),
            node_kind: ProjectTreeNodeKind::Step,
            label: step.meta.name.clone(),
            disabled: step.meta.disabled,
            parent_disabled: disabled || parent_disabled,
            depth: 2,
            leaf: true,
            expanded: false,
            first: ix == 0,
            last: ix == last_ix,
          }));
        }
      }
      TestCaseType::CaseStep { .. } => {
        case_tree_node.node_kind = ProjectTreeNodeKind::CaseStep;
        entries.push(case_tree_node);
      }
    }
  }
  entries
}

fn build_context_menu(node: &ProjectTreeNode) -> Box<ContextMenuBuilder> {
  let kind = node.node_kind;
  let disabled = node.disabled;
  let first = node.first;
  let last = node.last;
  let builder = move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
    menu
      .submenu_with_icon(Some(IconName::Plus.into()), "New", window, cx, move |submenu, _, _| {
        let mut submenu = submenu;
        if matches!(kind, ProjectTreeNodeKind::Suite) {
          submenu = submenu.menu_with_icon("Test Suite", IconAsset::TestSuite, Box::new(AddTestSuite));
        }
        if matches!(kind, ProjectTreeNodeKind::Suite | ProjectTreeNodeKind::Case) {
          submenu = submenu.menu_with_icon("Test Case", IconAsset::TestCase, Box::new(AddTestCase));
        }
        submenu.menu_with_icon("Test Step", IconAsset::TestStep, Box::new(AddTestStep))
      })
      .separator()
      .menu_with_icon("Duplicate", IconName::Copy, Box::new(Duplicate))
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
    let icon = node.icon();
    let left_padding = if icon.is_some() { px(0.) } else { px(14.) };

    h_flex()
      .id(node.id())
      .size_full()
      .pl(left_padding)
      .gap_x_1()
      .when_some(icon, |this, icon| this.child(icon))
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
      .on_action(cx.listener(Self::on_action_escape))
      .on_action(cx.listener(Self::on_action_switch_node_active_status))
      .on_action(cx.listener(Self::on_action_rename))
      .on_action(cx.listener(Self::on_action_delete))
      .on_action(cx.listener(Self::on_action_duplicate))
      .on_action(cx.listener(Self::on_action_add_test_suite))
      .on_action(cx.listener(Self::on_action_add_test_case))
      .on_action(cx.listener(Self::on_action_add_test_step))
      .on_action(cx.listener(Self::on_action_move_up))
      .on_action(cx.listener(Self::on_action_move_down))
      .size_full()
      .gap_y_2()
      .p_1()
      .child(
        h_flex()
          .gap_x_2()
          .w_full()
          .flex_row_reverse()
          .child(
            Button::new("btn-add-tree-node")
              .ghost()
              .small()
              .icon(IconName::Plus)
              .on_click({
                let tree_state = self.tree_state.clone();
                move |_, window, cx| {
                  tree_state.update(cx, |tree_state, cx| {
                    if let Some(new_path) = tree_state.delegate_mut().add_test_from_path(selected_path, window, cx) {
                      tree_state.set_selected_path(Some(new_path));
                    }
                  });
                }
              }),
          )
          .child(
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
