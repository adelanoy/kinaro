use crate::actions::{
  AddTestCase, AddTestStep, AddTestSuite, Duplicate, PROJECT_TREE_CONTEXT_KEY, RemoveNode, Rename, SwitchNodeActiveStatus,
};
use crate::ui::components::tree::{KiTree, KiTreeDelegate, KiTreeEvent, KiTreeState, ProjectTreeEntry};
use crate::ui::views::side_bar::project_configuration::ProjectConfigurationTab;
use crate::workspace::{Project, TestCase, TestNodeKind, TestStep, TestSuite};
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
use log::warn;
use std::collections::HashSet;
use uuid::Uuid;

type ContextMenuBuilder = dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu;

pub(super) struct ProjectTree {
  tree_state: Entity<KiTreeState<ProjectTreeDelegate>>,
  _tree_sub: Subscription,
}

impl ProjectTree {
  fn on_tree_event(
    &mut self,
    tree: &Entity<KiTreeState<ProjectTreeDelegate>>,
    e: &KiTreeEvent,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    tree.update(cx, |tree, cx| match e {
      KiTreeEvent::NodeExpanded(ix) => tree.delegate_mut().on_expand_status_change(*ix, true, cx),
      KiTreeEvent::NodeCollapsed(ix) => tree.delegate_mut().on_expand_status_change(*ix, false, cx),
      KiTreeEvent::EntryDoubleClicked(ix) => println!("Entry action: {}", ix),
    });
  }

  fn on_remove_node(&mut self, _action: &RemoveNode, _window: &mut Window, _cx: &mut Context<Self>) {}

  fn on_switch_active_status(&mut self, _action: &SwitchNodeActiveStatus, _window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(ix) = tree.selected_index() else {
        warn!("on_switch_active_status: no tree_state selected_ix");
        return;
      };
      tree.delegate_mut().on_switch_active_status(ix, cx);
    })
  }

  fn on_rename_entry(&mut self, _: &Rename, window: &mut Window, cx: &mut Context<Self>) {
    self.tree_state.update(cx, |tree, cx| {
      let Some(ix) = tree.selected_index() else {
        warn!("on_rename_entry: no tree_state selected_ix");
        return;
      };
      tree.delegate_mut().setup_rename_entry(ix, window, cx);
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
      let tree_state = cx.new(|cx| KiTreeState::new(ProjectTreeDelegate::new(project, window, cx), cx));
      let _tree_sub = cx.subscribe_in(&tree_state, window, Self::on_tree_event);

      Self { tree_state, _tree_sub }
    })
  }
}

struct ProjectTreeDelegate {
  project: Entity<Project>,
  tree_entries: Vec<ProjectTreeEntry>,
  rename_input_state: Entity<InputState>,
  _rename_input_sub: Option<Subscription>,
}

impl ProjectTreeDelegate {
  fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<KiTreeState<Self>>) -> Self {
    let rename_input_state = cx.new(|cx| InputState::new(window, cx));
    let mut delegate = Self {
      project,
      tree_entries: vec![],
      rename_input_state,
      _rename_input_sub: None,
    };
    delegate.update_tests_entries(cx);
    delegate
  }

  fn update_tests_entries(&mut self, cx: &mut Context<KiTreeState<Self>>) {
    self.tree_entries = self.project.read_with(cx, |project, cx| {
      let opened_nodes = project.opened_tree_nodes();
      get_ts_entries(&project.tests.read(cx).suites, opened_nodes)
    });
    cx.notify();
  }

  /// Updates the [`ProjectTreeEntry`] *open* property according to the provided value
  ///
  /// Also updates the project settings
  fn on_expand_status_change(&mut self, ix: usize, open: bool, cx: &mut Context<KiTreeState<Self>>) {
    let Some(entry) = self.tree_entries.get_mut(ix) else {
      warn!("on_expand_status_change: unknown item ix: {}", ix);
      return;
    };
    self.project.update(cx, move |project, cx| {
      if open {
        project.expand_tree_node(entry.id(), cx);
      } else {
        project.collapse_tree_node(&entry.id(), cx);
      }
    });
    // Implement a fine-grained update instead of rebuilding the full tree?
    self.update_tests_entries(cx);
  }

  fn on_switch_active_status(&mut self, ix: usize, cx: &mut Context<KiTreeState<Self>>) {
    let Some(entry) = self.tree_entries.get_mut(ix) else {
      warn!("on_switch_active_status: unknown item ix: {}", ix);
      return;
    };
    self.project.update(cx, move |project, cx| {
      project.tests.update(cx, |tests, cx| {
        tests.switch_active_status(&entry.path, cx);
      });
    });
    self.update_tests_entries(cx);
  }

  // Renaming
  fn setup_rename_entry(&mut self, ix: usize, window: &mut Window, cx: &mut Context<KiTreeState<Self>>) {
    let Some(entry) = self.tree_entries.get_mut(ix) else {
      warn!("on_rename_entry: unknown item ix: {}", ix);
      return;
    };
    let name = entry.label.clone();
    self.rename_input_state.update(cx, |input, cx| {
      input.set_value(name, window, cx);
      input.focus(window, cx);
    });
    self._rename_input_sub = Some(cx.subscribe_in(&self.rename_input_state, window, Self::on_rename_input_event));
  }
  fn on_rename_input_event(
    tree_state: &mut KiTreeState<Self>,
    input: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<KiTreeState<Self>>,
  ) {
    match event {
      InputEvent::PressEnter { .. } | InputEvent::Blur => {
        // Case: if Esc is pressed, selected_ix is None, and Blur event is raised, unwrap ix would panic
        if let Some(ix) = tree_state.selected_index() {
          tree_state.delegate_mut().rename_entry(ix, input.read(cx).value(), window, cx);
          tree_state.focus_handle(cx).focus(window, cx);
        }
        tree_state.delegate_mut()._rename_input_sub = None;
      }
      _ => {}
    }
  }

  fn rename_entry(&mut self, ix: usize, name: SharedString, window: &mut Window, cx: &mut Context<KiTreeState<Self>>) {
    let path = self.tree_entries[ix].path.clone();
    if let Err(err) = self.project.update(cx, |project, cx| {
      project.tests.update(cx, |tests, cx| tests.rename_at(&path, name, cx))
    }) {
      window.push_notification(err, cx);
    } else {
      self.update_tests_entries(cx);
    }
  }
}

fn get_ts_entries(suites: &[TestSuite], opened_nodes: &HashSet<Uuid>) -> Vec<ProjectTreeEntry> {
  let mut entries = Vec::new();
  for suite in suites.iter() {
    let id = suite.info.id;
    let leaf = suite.cases.is_empty();
    let expanded = opened_nodes.contains(&suite.info.id);
    let disabled = suite.info.disabled;
    entries.push(ProjectTreeEntry {
      path: vec![id],
      kind: TestNodeKind::Suite,
      label: suite.info.name.clone(),
      disabled,
      parent_disabled: false,
      depth: 0,
      leaf,
      expanded,
    });
    if !leaf && expanded {
      entries.extend(get_tc_entries(&suite.cases, id, disabled, opened_nodes));
    }
  }
  entries
}

fn get_tc_entries(cases: &[TestCase], parent_id: Uuid, parent_disabled: bool, opened_nodes: &HashSet<Uuid>) -> Vec<ProjectTreeEntry> {
  let mut entries = Vec::new();
  for case in cases.iter() {
    let is_empty = case.steps.is_empty();
    let path = vec![parent_id, case.info.id];
    if !case.is_anonymous {
      let is_open = opened_nodes.contains(&case.info.id);
      let disabled = case.info.disabled;
      entries.push(ProjectTreeEntry {
        path: path.clone(),
        kind: TestNodeKind::Case,
        label: case.info.name.clone(),
        disabled,
        parent_disabled,
        depth: 1,
        leaf: is_empty,
        expanded: is_open,
      });
      if !is_empty && is_open {
        entries.extend(get_steps_entries(&case.steps, path, disabled || parent_disabled));
      }
    } else if case.steps.len() == 1 {
      entries.push(get_anonymous_step_entry(&case.steps[0], path, parent_disabled));
    }
  }
  entries
}

fn get_steps_entries(cases: &[TestStep], parent_ix: Vec<Uuid>, parent_disabled: bool) -> Vec<ProjectTreeEntry> {
  cases
    .iter()
    .map(|step| {
      let mut path = parent_ix.clone();
      path.push(step.info.id);
      ProjectTreeEntry {
        path,
        kind: TestNodeKind::Step,
        label: step.info.name.clone(),
        disabled: step.info.disabled,
        parent_disabled,
        depth: 2,
        leaf: true,
        expanded: false,
      }
    })
    .collect()
}

fn get_anonymous_step_entry(step: &TestStep, parent_ix: Vec<Uuid>, parent_disabled: bool) -> ProjectTreeEntry {
  let mut path = parent_ix;
  path.push(step.info.id);
  ProjectTreeEntry {
    path,
    kind: TestNodeKind::AnonymousStep,
    label: step.info.name.clone(),
    disabled: step.info.disabled,
    parent_disabled,
    depth: 1,
    leaf: true,
    expanded: false,
  }
}

fn build_context_menu(disabled: bool, kind: TestNodeKind) -> Box<ContextMenuBuilder> {
  let builder = move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
    menu
      .submenu_with_icon(Some(IconName::Plus.into()), "New", window, cx, move |submenu, _, _| {
        let mut submenu = submenu;
        if matches!(kind, TestNodeKind::Suite) {
          submenu = submenu.menu("Test Suite", Box::new(AddTestSuite));
        }
        if matches!(kind, TestNodeKind::Suite) || matches!(kind, TestNodeKind::Case) {
          submenu = submenu.menu("Test Case", Box::new(AddTestCase));
        }
        submenu.menu("Test Step", Box::new(AddTestStep))
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

impl KiTreeDelegate for ProjectTreeDelegate {
  fn row_count(&self, _cx: &App) -> usize {
    self.tree_entries.len()
  }

  fn entry(&self, ix: usize, _cx: &Context<KiTreeState<Self>>) -> &ProjectTreeEntry {
    self.tree_entries.get(ix).unwrap()
  }

  fn entry_render(&self, ix: usize, selected: bool, _window: &mut Window, cx: &mut Context<KiTreeState<Self>>) -> impl IntoElement {
    let entry = &self.tree_entries[ix];
    let icon = entry.icon();
    let is_edited = selected && self._rename_input_sub.is_some();
    let left_padding = if icon.is_some() { px(0.) } else { px(14.) };

    h_flex()
      .id(entry.id())
      .size_full()
      .pl(left_padding)
      .gap_x_1()
      .when_some(icon, |this, icon| this.child(icon))
      .when_else(
        is_edited,
        |this| this.child(Input::new(&self.rename_input_state)),
        |this| {
          this
            .child(entry.label.clone())
            .when(entry.disabled || entry.parent_disabled, |this| {
              this.child(Icon::new(IconAsset::Ban).xsmall())
            })
            .when(entry.disabled, |this| this.text_color(cx.theme().muted_foreground))
        },
      )
      .context_menu(build_context_menu(entry.disabled, entry.kind))
  }
}

impl Render for ProjectTree {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let entry_selected = self.tree_state.read(cx).selected_index().is_some();
    v_flex()
      .id("project-tree")
      .key_context(PROJECT_TREE_CONTEXT_KEY)
      .on_action(cx.listener(Self::on_remove_node))
      .on_action(cx.listener(Self::on_switch_active_status))
      .on_action(cx.listener(Self::on_rename_entry))
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
              .disabled(!entry_selected)
              .icon(IconName::Delete),
          ),
      )
      .child(KiTree::new(self.tree_state.clone()))
  }
}
