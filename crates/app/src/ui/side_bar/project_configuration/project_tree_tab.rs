use crate::actions::{
  AddTestCase, AddTestStep, AddTestSuite, Duplicate, PROJECT_TREE_CONTEXT_KEY, RemoveNode, Rename, SwitchNodeActiveStatus,
};
use crate::ui::components::tree::{ProjectTreeNode, Tree, TreeDelegate, TreeEvent, TreeState};
use crate::ui::side_bar::project_configuration::ProjectConfigurationTab;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{
  ActiveTheme, Disableable, Icon, IconName, Sizable, WindowExt,
  button::{Button, ButtonVariants},
  h_flex,
  input::{Input, InputEvent, InputState},
  menu::{ContextMenuExt, PopupMenu},
  v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use ki_assets::icon::IconAsset;
use ki_workspace::test::test_case::TestCaseType;
use ki_workspace::test::{TestInfoId, TestsContainer};
use ki_workspace::{Project, TestCase, TestStep, TestSuite};
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
      TreeEvent::NodeExpanded(ix) => tree.delegate_mut().on_expand_status_change(*ix, true, cx),
      TreeEvent::NodeCollapsed(ix) => tree.delegate_mut().on_expand_status_change(*ix, false, cx),
      TreeEvent::EntryDoubleClicked(ix) => println!("Entry action: {}", ix),
    });
  }

  fn on_remove_node(&mut self, _action: &RemoveNode, _window: &mut Window, _cx: &mut Context<Self>) {}

  fn on_switch_node_enable_status(&mut self, _action: &SwitchNodeActiveStatus, _window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(ix) = tree.selected_index() else {
        warn!("ProjectTree:on_switch_node_enable_status: no tree_state selected_ix");
        return;
      };
      tree.delegate_mut().on_switch_node_enable_status(ix, cx);
    })
  }

  fn on_rename_node(&mut self, _: &Rename, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(ix) = tree.selected_index() else {
        warn!("ProjectTree:on_rename_node: no tree_state selected_ix");
        return;
      };
      tree.delegate_mut().setup_rename_node(ix, window, cx);
    });
  }

  fn on_duplicate_node(&mut self, _: &Duplicate, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(ix) = tree.selected_index() else {
        warn!("ProjectTree:on_duplicate_node: no tree_state selected_ix");
        return;
      };
      tree.delegate_mut().duplicate_node(ix, window, cx);
    });
  }

  fn on_add_test_case(&mut self, _: &AddTestCase, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(ix) = tree.selected_index() else {
        warn!("ProjectTree:on_add_step: no tree_state selected_ix");
        return;
      };
      tree.delegate_mut().add_test_case(ix, window, cx);
    });
  }

  fn on_add_test_step(&mut self, _: &AddTestStep, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(ix) = tree.selected_index() else {
        warn!("ProjectTree:on_add_step: no tree_state selected_ix");
        return;
      };
      tree.delegate_mut().add_test_step(ix, window, cx);
    });
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
  rename_input_state: Entity<InputState>,
  _rename_input_sub: Option<Subscription>,
}

impl ProjectTreeDelegate {
  fn new(tests: Entity<TestsContainer>, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> Self {
    let rename_input_state = cx.new(|cx| InputState::new(window, cx));
    let mut delegate = Self {
      tests,
      tree_nodes: vec![],
      rename_input_state,
      _rename_input_sub: None,
    };
    delegate.update_tests_nodes(cx);
    delegate
  }

  fn update_tests_nodes(&mut self, cx: &mut Context<TreeState<Self>>) {
    self.tree_nodes = self.tests.read_with(cx, |tests, _| {
      let opened_nodes = tests.opened_tree_nodes();
      get_ts_entries(&tests.suites, opened_nodes)
    });
    cx.notify();
  }

  fn on_expand_status_change(&mut self, ix: usize, open: bool, cx: &mut Context<TreeState<Self>>) {
    let Some(id) = self.tree_nodes.get(ix).map(|node| node.id()) else {
      warn!("ProjectTreeDelegate:on_expand_status_change: unknown item ix: {}", ix);
      return;
    };
    self.tests.update(cx, move |project, cx| {
      if open {
        project.expand_tree_node(id, cx);
      } else {
        project.collapse_tree_node(&id, cx);
      }
    });
    // Implement a fine-grained update instead of rebuilding the full tree?
    self.update_tests_nodes(cx);
  }

  fn on_switch_node_enable_status(&mut self, ix: usize, cx: &mut Context<TreeState<Self>>) {
    let Some(info_id) = self.tree_nodes.get(ix).map(|node| &node.info_id) else {
      warn!("ProjectTreeDelegate:on_switch_active_status: unknown item ix: {}", ix);
      return;
    };
    self.tests.update(cx, move |tests, cx| {
      tests.switch_node_enable_status(info_id, cx);
    });
    self.update_tests_nodes(cx);
  }

  // Renaming
  fn setup_rename_node(&mut self, ix: usize, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(name) = self.tree_nodes.get(ix).map(|node| node.label.clone()) else {
      warn!("ProjectTreeDelegate:setup_rename_node: unknown item ix: {}", ix);
      return;
    };
    self.rename_input_state.update(cx, |input, cx| {
      input.set_value(name, window, cx);
      input.focus(window, cx);
    });
    self._rename_input_sub = Some(cx.subscribe_in(&self.rename_input_state, window, Self::on_rename_input_event));
  }

  fn on_rename_input_event(
    tree_state: &mut TreeState<Self>,
    input: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<TreeState<Self>>,
  ) {
    match event {
      InputEvent::PressEnter { .. } | InputEvent::Blur => {
        // Case: if Esc is pressed, selected_ix is None, and Blur event is raised, unwrap ix would panic
        if let Some(ix) = tree_state.selected_index() {
          tree_state.delegate_mut().rename_node(ix, input.read(cx).value(), window, cx);
          tree_state.focus_handle(cx).focus(window, cx);
        }
        tree_state.delegate_mut()._rename_input_sub = None;
      }
      _ => {}
    }
  }

  fn rename_node(&mut self, ix: usize, name: SharedString, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(info_id) = self.tree_nodes.get(ix).map(|node| &node.info_id) else {
      warn!("ProjectTreeDelegate:rename_node: unknown item ix: {}", ix);
      return;
    };
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.rename_at(info_id, name, cx)) {
      window.push_notification(err, cx);
    } else {
      self.update_tests_nodes(cx);
    }
  }

  fn duplicate_node(&mut self, ix: usize, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(info_id) = self.tree_nodes.get(ix).map(|node| &node.info_id) else {
      warn!("ProjectTreeDelegate:duplicate_node: unknown item ix: {}", ix);
      return;
    };
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.duplicate_test(info_id, cx)) {
      window.push_notification(err, cx);
    } else {
      self.update_tests_nodes(cx);
    }
  }

  fn add_test_case(&mut self, ix: usize, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(info_id) = self.tree_nodes.get(ix).map(|node| &node.info_id) else {
      warn!("ProjectTreeDelegate:add_test_case: unknown item ix: {}", ix);
      return;
    };
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.add_test_case(info_id, cx)) {
      window.push_notification(err, cx);
    } else {
      self.update_tests_nodes(cx);
    }
  }

  fn add_test_step(&mut self, ix: usize, window: &mut Window, cx: &mut Context<TreeState<Self>>) {
    let Some(info_id) = self.tree_nodes.get(ix).map(|node| &node.info_id) else {
      warn!("ProjectTreeDelegate:add_test_step: unknown item ix: {}", ix);
      return;
    };
    if let Err(err) = self.tests.update(cx, |tests, cx| tests.add_test_step(info_id, cx)) {
      window.push_notification(err, cx);
    } else {
      self.update_tests_nodes(cx);
    }
  }
}

fn get_ts_entries(suites: &[TestSuite], opened_nodes: &HashSet<Uuid>) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  for suite in suites.iter() {
    let leaf = suite.cases.is_empty();
    let info_id = suite.info.info_id();
    let disabled = suite.info.disabled;
    let expanded = opened_nodes.contains(&info_id.id());
    entries.push(ProjectTreeNode {
      info_id,
      label: suite.info.name.clone(),
      disabled,
      parent_disabled: false,
      depth: 0,
      leaf,
      expanded,
    });
    if !leaf && expanded {
      entries.extend(get_tc_entries(&suite.cases, disabled, opened_nodes));
    }
  }
  entries
}

fn get_tc_entries(cases: &[TestCase], parent_disabled: bool, opened_nodes: &HashSet<Uuid>) -> Vec<ProjectTreeNode> {
  let mut entries = Vec::new();
  cases.iter().for_each(|case| {
    let info_id = case.info.info_id();
    let disabled = case.info.disabled;
    let mut case_tree_node = ProjectTreeNode {
      info_id,
      label: case.info.name.clone(),
      disabled,
      parent_disabled,
      depth: 1,
      leaf: true,
      expanded: false,
    };
    match case.case_type() {
      TestCaseType::CaseMulti { steps } => {
        let is_empty = steps.is_empty();
        let is_open = opened_nodes.contains(&info_id.id());
        case_tree_node.leaf = is_empty;
        case_tree_node.expanded = is_open;
        entries.push(case_tree_node);
        if !is_empty && is_open {
          entries.extend(steps.iter().map(|step| get_step_node(step, disabled || parent_disabled)));
        }
      }
      TestCaseType::CaseStep { .. } => {
        entries.push(case_tree_node);
      }
    }
  });
  entries
}

fn get_step_node(step: &TestStep, parent_disabled: bool) -> ProjectTreeNode {
  ProjectTreeNode {
    info_id: step.info.info_id(),
    label: step.info.name.clone(),
    disabled: step.info.disabled,
    parent_disabled,
    depth: 2,
    leaf: true,
    expanded: false,
  }
}

fn build_context_menu(disabled: bool, kind: TestInfoId) -> Box<ContextMenuBuilder> {
  let builder = move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
    menu
      .submenu_with_icon(Some(IconName::Plus.into()), "New", window, cx, move |submenu, _, _| {
        let mut submenu = submenu;
        if matches!(kind, TestInfoId::Suite(_)) {
          submenu = submenu.menu_with_icon("Test Suite", IconAsset::TestSuite, Box::new(AddTestSuite));
        }
        if matches!(kind, TestInfoId::Suite(_)) || matches!(kind, TestInfoId::CaseMulti(_, _)) {
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
      .menu_with_icon("Remove", IconName::Delete, Box::new(RemoveNode))
  };
  Box::new(builder)
}

impl TreeDelegate for ProjectTreeDelegate {
  fn row_count(&self, _cx: &App) -> usize {
    self.tree_nodes.len()
  }

  fn node(&self, ix: usize, _cx: &Context<TreeState<Self>>) -> &ProjectTreeNode {
    self.tree_nodes.get(ix).unwrap()
  }

  fn node_render(&self, ix: usize, selected: bool, _window: &mut Window, cx: &mut Context<TreeState<Self>>) -> impl IntoElement {
    let node = &self.tree_nodes[ix];
    let id = node.info_id.id();
    let icon = node.icon();
    let is_edited = selected && self._rename_input_sub.is_some();
    let left_padding = if icon.is_some() { px(0.) } else { px(14.) };

    h_flex()
      .id(node.id())
      .size_full()
      .pl(left_padding)
      .gap_x_1()
      .when_some(icon, |this, icon| this.child(icon))
      .when(!is_edited, |this| {
        this.tooltip(move |window, cx| Tooltip::new(format!("{}", id.clone())).build(window, cx))
      })
      .when_else(
        is_edited,
        |this| this.child(Input::new(&self.rename_input_state)),
        |this| {
          this
            .child(node.label.clone())
            .when(node.disabled || node.parent_disabled, |this| {
              this.child(Icon::new(IconAsset::Ban).xsmall())
            })
            .when(node.disabled, |this| this.text_color(cx.theme().muted_foreground))
        },
      )
      .context_menu(build_context_menu(node.disabled, node.info_id))
  }
}

impl Render for ProjectTree {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let node_selected = self.tree_state.read(cx).selected_index().is_some();
    v_flex()
      .id("project-tree")
      .key_context(PROJECT_TREE_CONTEXT_KEY)
      .on_action(cx.listener(Self::on_remove_node))
      .on_action(cx.listener(Self::on_switch_node_enable_status))
      .on_action(cx.listener(Self::on_rename_node))
      .on_action(cx.listener(Self::on_duplicate_node))
      .on_action(cx.listener(Self::on_add_test_case))
      .on_action(cx.listener(Self::on_add_test_step))
      .size_full()
      .gap_y_2()
      .p_1()
      .child(
        h_flex()
          .gap_x_2()
          .w_full()
          .flex_row_reverse()
          .child(Button::new("btn-add-tree-node").ghost().small().icon(IconName::Plus))
          .child(
            Button::new("btn-remove-tree-node")
              .ghost()
              .small()
              .disabled(!node_selected)
              .icon(IconName::Delete),
          ),
      )
      .child(Tree::new(self.tree_state.clone()))
  }
}
