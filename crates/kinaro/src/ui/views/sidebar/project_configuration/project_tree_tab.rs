use crate::ui::components::tree::{KiTree, KiTreeDelegate, KiTreeEntry, KiTreeEvent, KiTreeState};
use crate::ui::views::sidebar::project_configuration::ProjectConfigurationTab;
use crate::workspace::Project;
use crate::workspace::test::{TestCase, TestNodeKind, TestStep, TestSuite};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::menu::{ContextMenuExt, PopupMenu};
use gpui_component::{ActiveTheme, Icon, IconName, h_flex, v_flex};
use ki_assets::icon::IconAsset;
use log::warn;
use std::collections::HashSet;
use uuid::Uuid;

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

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
struct SwitchNodeActiveStatus(usize);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = profile, no_json)]
struct DisableNode(usize);

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
            KiTreeEvent::EntryClicked(_) => {}
            KiTreeEvent::EntryDoubleClicked(_) => {}
            KiTreeEvent::NodeStateChanged(ix, status) => tree
                .delegate_mut()
                .change_node_open_status(*ix, *status, cx),
        });
    }
    fn on_remove_node(action: &RemoveNode, _window: &mut Window, _cx: &mut App) {
        println!("Removed node {}", action.0);
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

            Self {
                tree_state,
                _tree_sub,
            }
        })
    }
}

struct ProjectTreeDelegate {
    project: Entity<Project>,
    tree_entries: Vec<KiTreeEntry>,
}

impl ProjectTreeDelegate {
    fn new(project: Entity<Project>, cx: &Context<KiTreeState<Self>>) -> Self {
        let mut delegate = Self {
            project,
            tree_entries: vec![],
        };
        delegate.update_tests_entries(cx);
        delegate
    }

    fn update_tests_entries(&mut self, cx: &Context<KiTreeState<Self>>) {
        self.tree_entries = self.project.read_with(cx, |project, cx| {
            let opened_nodes = project.opened_tree_nodes();
            Self::get_ts_entries(&project.tests.read(cx).test_suites, opened_nodes)
        });
    }

    fn get_ts_entries(suites: &[TestSuite], opened_nodes: &HashSet<Uuid>) -> Vec<KiTreeEntry> {
        let mut entries = Vec::new();
        for suite in suites.iter() {
            let is_empty = suite.cases.is_empty();
            let is_open = opened_nodes.contains(&suite.info.id);
            entries.push(KiTreeEntry {
                id: suite.info.id,
                kind: TestNodeKind::Suite,
                label: suite.info.name.clone(),
                disabled: suite.info.active,
                depth: 0,
                leaf: is_empty,
                open: is_open,
            });
            if !is_empty && is_open {
                entries.extend(Self::get_tc_entries(&suite.cases, opened_nodes));
            }
        }
        entries
    }

    fn get_tc_entries(cases: &[TestCase], opened_nodes: &HashSet<Uuid>) -> Vec<KiTreeEntry> {
        let mut entries = Vec::new();
        for case in cases.iter() {
            let is_empty = case.steps.is_empty();
            if !case.is_anonymous {
                let is_open = opened_nodes.contains(&case.info.id);
                entries.push(KiTreeEntry {
                    id: case.info.id,
                    kind: TestNodeKind::Case,
                    label: case.info.name.clone(),
                    disabled: case.info.active,
                    depth: 1,
                    leaf: is_empty,
                    open: is_open,
                });
                if !is_empty && is_open {
                    entries.extend(Self::get_steps_entries(&case.steps, 2));
                }
            } else {
                entries.extend(Self::get_steps_entries(&case.steps, 1));
            }
        }
        entries
    }

    fn get_steps_entries(cases: &[TestStep], depth: usize) -> Vec<KiTreeEntry> {
        cases
            .iter()
            .map(|case| KiTreeEntry {
                id: case.info.id,
                kind: TestNodeKind::Step,
                label: case.info.name.clone(),
                disabled: case.info.active,
                depth,
                leaf: true,
                open: false,
            })
            .collect()
    }

    fn entry_icon(&self, entry: &KiTreeEntry) -> Option<Icon> {
        match entry.kind {
            TestNodeKind::Suite => Some(Icon::new(IconName::FolderOpen)),
            TestNodeKind::Case => Some(Icon::new(IconAsset::Variable)),
            TestNodeKind::Step => None,
        }
    }

    /// Updates the [`KiTreeEntry`] *open* property according to the provided value
    ///
    /// Also updates the project settings
    fn change_node_open_status(
        &mut self,
        ix: usize,
        open: bool,
        cx: &mut Context<KiTreeState<Self>>,
    ) {
        let Some(entry) = self.tree_entries.get_mut(ix) else {
            warn!(
                "Received a ProjectTree change node event on unknown ix: {}",
                ix
            );
            return;
        };
        self.project.update(cx, move |project, cx| {
            if open {
                project.add_opened_tree_node(entry.id, cx);
                entry.open = true;
            } else {
                project.remove_opened_tree_node(&entry.id, cx);
                entry.open = false;
            }
        });
        // Implement a fine-grained update instead of rebuilding the full tree?
        self.update_tests_entries(cx);
    }

    fn build_context_menu(
        ix: usize,
        disabled: bool,
        kind: TestNodeKind,
    ) -> Box<ContextMenuBuilder> {
        let builder = move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
            let mut menu = menu
                .submenu_with_icon(
                    Some(IconAsset::Plus.into()),
                    "Add",
                    window,
                    cx,
                    move |submenu, _, _| {
                        let mut submenu = submenu;
                        if matches!(kind, TestNodeKind::Suite) {
                            submenu = submenu.menu("Test Suite", Box::new(AddTestSuite(ix)));
                        }
                        if matches!(kind, TestNodeKind::Suite) || matches!(kind, TestNodeKind::Case)
                        {
                            submenu = submenu.menu("Test Case", Box::new(AddTestCase(ix)));
                        }
                        submenu.menu("Test Step", Box::new(AddTestStep(ix)))
                    },
                )
                .menu_with_icon("Remove", IconName::Delete, Box::new(RemoveNode(ix)))
                .menu_with_icon("Duplicate", IconName::Copy, Box::new(DuplicateNode(ix)))
                .menu_with_check("Enabled", !disabled, Box::new(SwitchNodeActiveStatus(ix)));

            menu
        };

        Box::new(builder)
    }
}

impl KiTreeDelegate for ProjectTreeDelegate {
    fn row_count(&self, _cx: &App) -> usize {
        self.tree_entries.len()
    }

    fn entry(&self, ix: usize, _cx: &Context<KiTreeState<Self>>) -> KiTreeEntry {
        self.tree_entries.get(ix).unwrap().clone()
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
            .when_some(icon, |this, icon| this.child(icon))
            .child(entry.label.clone())
            .when(entry.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .context_menu(Self::build_context_menu(ix, entry.disabled, entry.kind))
    }
}

impl Render for ProjectTree {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("project-tree")
            .on_action(Self::on_remove_node)
            .size_full()
            .p_1()
            .child(KiTree::new(self.tree_state.clone()))
    }
}
