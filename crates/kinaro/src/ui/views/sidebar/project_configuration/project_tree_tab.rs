use crate::ui::components::tree::{
    KiTree, KiTreeDelegate, KiTreeEvent, KiTreeState, ProjectTreeEntry,
};
use crate::ui::views::sidebar::project_configuration::ProjectConfigurationTab;
use crate::workspace::Project;
use crate::workspace::test::{TestCase, TestNodeKind, TestStep, TestSuite};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{ContextMenuExt, PopupMenu};
use gpui_component::{ActiveTheme, Disableable, Icon, IconName, Sizable, h_flex, v_flex};
use ki_assets::icon::IconAsset;
use log::warn;
use std::collections::HashSet;
use uuid::Uuid;

pub const PROJECT_TREE_CONTEXT_KEY: &str = "ProjectTree";

type ContextMenuBuilder = dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu;

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
struct RemoveNode(usize);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
struct AddTestSuite(usize);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
struct AddTestCase(usize);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
struct AddTestStep(usize);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
struct DuplicateNode(usize);

actions!([SwitchNodeActiveStatus]);

pub(super) struct ProjectTree {
    tree_state: Entity<KiTreeState<ProjectTreeDelegate>>,
    focus_handle: FocusHandle,
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
            KiTreeEvent::NodeExpanded(ix) => {
                tree.delegate_mut().on_expand_status_change(*ix, true, cx)
            }
            KiTreeEvent::NodeCollapsed(ix) => {
                tree.delegate_mut().on_expand_status_change(*ix, false, cx)
            }
            _ => {}
        });
    }

    fn on_remove_node(
        &mut self,
        action: &RemoveNode,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        println!("Removed node {}", action.0);
    }

    fn on_switch_active_status(
        &mut self,
        _action: &SwitchNodeActiveStatus,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tree_state.update(cx, |tree, cx| {
            let Some(ix) = tree.selected_index() else {
                warn!("on_switch_active_status: no tree_state selected_ix");
                return;
            };
            tree.delegate_mut().on_switch_active_status(ix, cx);
        })
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
            let tree_state =
                cx.new(|cx| KiTreeState::new(ProjectTreeDelegate::new(project, cx), cx));
            let _tree_sub = cx.subscribe_in(&tree_state, window, Self::on_tree_event);

            cx.bind_keys([KeyBinding::new(
                "ctrl-shift-d",
                SwitchNodeActiveStatus,
                Some(PROJECT_TREE_CONTEXT_KEY),
            )]);

            Self {
                tree_state,
                focus_handle: cx.focus_handle(),
                _tree_sub,
            }
        })
    }
}

struct ProjectTreeDelegate {
    project: Entity<Project>,
    tree_entries: Vec<ProjectTreeEntry>,
}

impl ProjectTreeDelegate {
    fn new(project: Entity<Project>, cx: &mut Context<KiTreeState<Self>>) -> Self {
        let mut delegate = Self {
            project,
            tree_entries: vec![],
        };
        delegate.update_tests_entries(cx);
        delegate
    }

    fn update_tests_entries(&mut self, cx: &mut Context<KiTreeState<Self>>) {
        self.tree_entries = self.project.read_with(cx, |project, cx| {
            let opened_nodes = project.opened_tree_nodes();
            Self::get_ts_entries(&project.tests.read(cx).suites, opened_nodes)
        });
        cx.notify();
    }

    fn get_ts_entries(suites: &[TestSuite], opened_nodes: &HashSet<Uuid>) -> Vec<ProjectTreeEntry> {
        let mut entries = Vec::new();
        for (ix, suite) in suites.iter().enumerate() {
            let leaf = suite.cases.is_empty();
            let expanded = opened_nodes.contains(&suite.info.id);
            let disabled = suite.info.disabled;
            entries.push(ProjectTreeEntry {
                id: suite.info.id,
                path: vec![ix],
                kind: TestNodeKind::Suite,
                label: suite.info.name.clone(),
                disabled,
                parent_disabled: false,
                depth: 0,
                leaf,
                expanded,
            });
            if !leaf && expanded {
                entries.extend(Self::get_tc_entries(
                    &suite.cases,
                    ix,
                    disabled,
                    opened_nodes,
                ));
            }
        }
        entries
    }

    fn get_tc_entries(
        cases: &[TestCase],
        parent_ix: usize,
        parent_disabled: bool,
        opened_nodes: &HashSet<Uuid>,
    ) -> Vec<ProjectTreeEntry> {
        let mut entries = Vec::new();
        for (ix, case) in cases.iter().enumerate() {
            let is_empty = case.steps.is_empty();
            let path = vec![parent_ix, ix];
            if !case.is_anonymous {
                let is_open = opened_nodes.contains(&case.info.id);
                let disabled = case.info.disabled;
                entries.push(ProjectTreeEntry {
                    id: case.info.id,
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
                    entries.extend(Self::get_steps_entries(
                        &case.steps,
                        path,
                        disabled || parent_disabled,
                    ));
                }
            } else if case.steps.len() == 1 {
                entries.push(Self::get_anonymous_step_entry(
                    &case.steps[0],
                    path,
                    parent_disabled,
                ));
            }
        }
        entries
    }

    fn get_steps_entries(
        cases: &[TestStep],
        parent_ix: Vec<usize>,
        parent_disabled: bool,
    ) -> Vec<ProjectTreeEntry> {
        cases
            .iter()
            .enumerate()
            .map(|(ix, case)| {
                let mut path = parent_ix.clone();
                path.push(ix);
                ProjectTreeEntry {
                    id: case.info.id,
                    path,
                    kind: TestNodeKind::Step,
                    label: case.info.name.clone(),
                    disabled: case.info.disabled,
                    parent_disabled,
                    depth: 2,
                    leaf: true,
                    expanded: false,
                }
            })
            .collect()
    }

    fn get_anonymous_step_entry(
        step: &TestStep,
        parent_ix: Vec<usize>,
        parent_disabled: bool,
    ) -> ProjectTreeEntry {
        let mut path = parent_ix;
        path.push(0);
        ProjectTreeEntry {
            id: step.info.id,
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

    fn entry_icon(&self, entry: &ProjectTreeEntry) -> Option<Icon> {
        match entry.kind {
            TestNodeKind::Suite => Some(Icon::new(IconAsset::TestSuite)),
            TestNodeKind::Case => Some(Icon::new(IconAsset::TestCase)),
            TestNodeKind::Step | TestNodeKind::AnonymousStep => {
                Some(Icon::new(IconAsset::TestStep))
            }
        }
    }

    /// Updates the [`ProjectTreeEntry`] *open* property according to the provided value
    ///
    /// Also updates the project settings
    fn on_expand_status_change(
        &mut self,
        ix: usize,
        open: bool,
        cx: &mut Context<KiTreeState<Self>>,
    ) {
        let Some(entry) = self.tree_entries.get_mut(ix) else {
            warn!("on_expand_status_change: unknown item ix: {}", ix);
            return;
        };
        self.project.update(cx, move |project, cx| {
            if open {
                project.expand_tree_node(entry.id, cx);
            } else {
                project.collapse_tree_node(&entry.id, cx);
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

    fn build_context_menu(
        ix: usize,
        disabled: bool,
        kind: TestNodeKind,
    ) -> Box<ContextMenuBuilder> {
        let builder = move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
            menu.submenu_with_icon(
                Some(IconName::Plus.into()),
                "Add",
                window,
                cx,
                move |submenu, _, _| {
                    let mut submenu = submenu;
                    if matches!(kind, TestNodeKind::Suite) {
                        submenu = submenu.menu("Test Suite", Box::new(AddTestSuite(ix)));
                    }
                    if matches!(kind, TestNodeKind::Suite) || matches!(kind, TestNodeKind::Case) {
                        submenu = submenu.menu("Test Case", Box::new(AddTestCase(ix)));
                    }
                    submenu.menu("Test Step", Box::new(AddTestStep(ix)))
                },
            )
            .menu_with_icon("Remove", IconName::Delete, Box::new(RemoveNode(ix)))
            .menu_with_icon("Duplicate", IconName::Copy, Box::new(DuplicateNode(ix)))
            .menu_with_check("Enabled", !disabled, Box::new(SwitchNodeActiveStatus))
        };

        Box::new(builder)
    }
}

impl KiTreeDelegate for ProjectTreeDelegate {
    fn row_count(&self, _cx: &App) -> usize {
        self.tree_entries.len()
    }

    fn entry(&self, ix: usize, _cx: &Context<KiTreeState<Self>>) -> &ProjectTreeEntry {
        self.tree_entries.get(ix).unwrap()
    }

    fn entry_render(
        &self,
        ix: usize,
        _window: &mut Window,
        cx: &mut Context<KiTreeState<Self>>,
    ) -> impl IntoElement {
        let entry = &self.tree_entries[ix];
        let icon = self.entry_icon(entry);
        let left_padding = if icon.is_some() { px(0.) } else { px(14.) };

        h_flex()
            .id(entry.id)
            .size_full()
            .pl(left_padding)
            .gap_x_1()
            .when_some(icon, |this, icon| this.child(icon))
            .child(entry.label.clone())
            .when(entry.disabled || entry.parent_disabled, |this| {
                this.child(Icon::new(IconAsset::Ban).xsmall())
            })
            .when(entry.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .context_menu(Self::build_context_menu(ix, entry.disabled, entry.kind))
    }
}

impl Render for ProjectTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entry_selected = self.tree_state.read(cx).selected_index().is_some();
        v_flex()
            .id("project-tree")
            .key_context(PROJECT_TREE_CONTEXT_KEY)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_remove_node))
            .on_action(cx.listener(Self::on_switch_active_status))
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
                            .icon(IconName::Plus),
                    )
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
