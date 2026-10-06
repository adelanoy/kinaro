mod case_editor;
mod header_editor;
mod profile_selector;
mod suite_editor;

use crate::ui::ProjectTestNodeKind;
use crate::ui::editor::case_editor::CaseEditor;
use crate::ui::editor::profile_selector::ProfileSelector;
use crate::ui::editor::suite_editor::SuiteEditor;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Icon, IconName, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
  AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div,
};
use ki_utils::TestPath;
use ki_workspace::Project;
use ki_workspace::test::{TestsContainer, TestsContainerEvent};
use log::warn;

/// Display data of a tab opened in the editor
struct TabData {
  path: TestPath,
  name: SharedString,
  kind: ProjectTestNodeKind,
}

/// The editor shown for the active tab, chosen from the kind of its node.
enum ActiveEditor {
  Suite(Entity<SuiteEditor>),
  Case(Entity<CaseEditor>),
  /// Placeholder for the node kinds that don't have an editor yet.
  Wip,
}

/// The active tab, along with its editor.
struct ActiveTab {
  index: usize,
  path: TestPath,
  editor: ActiveEditor,
}

impl ActiveTab {
  /// Builds the editor of `tab`, located at `index` in the tab list.
  fn new(index: usize, tab: &TabData, tests: &Entity<TestsContainer>, window: &mut Window, cx: &mut Context<EditorView>) -> Self {
    let editor = match tab.kind {
      ProjectTestNodeKind::Suite => ActiveEditor::Suite(cx.new(|cx| SuiteEditor::new(tests.clone(), tab.path, window, cx))),
      ProjectTestNodeKind::Case => ActiveEditor::Case(cx.new(|cx| CaseEditor::new(tests.clone(), tab.path, window, cx))),
      ProjectTestNodeKind::CaseStep | ProjectTestNodeKind::Step => ActiveEditor::Wip,
    };
    Self {
      index,
      path: tab.path,
      editor,
    }
  }
}

/// The main editor view: one tab per test node opened from the project tree.
///
/// The opened tabs and the active one are mirrored in the project's [`TestsContainer`] so they
/// are persisted in the workspace, and kept in sync with the test tree (renames, moves, removals).
pub struct EditorView {
  tests: Entity<TestsContainer>,
  tabs: Vec<TabData>,
  profile_selector: Entity<ProfileSelector>,
  active_tab: Option<ActiveTab>,
  _tests_sub: Subscription,
}

impl EditorView {
  /// Closes the tab at `closed_ix`, selecting the next tab (or the previous one when it was the
  /// last) if it was the active one.
  fn close_tab(&mut self, closed_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
    let tab = self.tabs.remove(closed_ix);
    self.tests.update(cx, |tests, cx| tests.close_test_from_editor(tab.path, cx));
    let new_active_tab_ix = match self.active_tab.as_ref().map(|active_tab| active_tab.index) {
      _ if self.tabs.is_empty() => None,
      Some(current_ix) if current_ix > closed_ix => Some(current_ix - 1),
      Some(current_ix) => Some(current_ix.min(self.tabs.len() - 1)),
      None => None,
    };
    self.select_tab(new_active_tab_ix, window, cx);
  }

  /// Builds the tabs from the ones saved in `tests`, making sure there is an active tab index if
  /// and only if there are tabs.
  fn get_tabs(tests: &Entity<TestsContainer>, window: &mut Window, cx: &mut Context<Self>) -> (Vec<TabData>, Option<ActiveTab>) {
    let (tabs, mut ix) = tests.read_with(cx, |tests, _| {
      (
        tests
          .opened_editor_nodes()
          .iter()
          .filter_map(|path| Self::tab_from_path(tests, path))
          .collect::<Vec<_>>(),
        tests.active_editor_tab_index(),
      )
    });
    // Check coherency of index
    if !tabs.is_empty() && ix.is_none_or(|ix| ix >= tabs.len()) {
      ix = Some(0);
      tests.update(cx, |tests, cx| tests.set_active_editor_tab(ix, cx));
    }
    if tabs.is_empty() && ix.is_some() {
      ix = None;
      tests.update(cx, |tests, cx| tests.set_active_editor_tab(ix, cx));
    }

    let active_tab = ix.map(|ix| ActiveTab::new(ix, &tabs[ix], tests, window, cx));
    (tabs, active_tab)
  }

  /// Creates the editor of `project`, restoring the tabs saved in its tests.
  pub fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let tests = project.read(cx).tests.clone();
    let _tests_sub = cx.subscribe_in(&tests, window, Self::tests_event_listener);
    let (tabs, active_tab) = Self::get_tabs(&tests, window, cx);
    let vars = project.read(cx).variables.clone();
    let profile_selector = cx.new(|cx| ProfileSelector::new(vars, window, cx));
    Self {
      tests,
      tabs,
      profile_selector,
      active_tab,
      _tests_sub,
    }
  }

  /// Keeps the tabs in sync with the test tree: opens (or selects) a tab on open requests, and
  /// updates the tabs of renamed, moved or removed nodes.
  fn tests_event_listener(
    &mut self,
    tests: &Entity<TestsContainer>,
    event: &TestsContainerEvent,
    window: &mut Window,
    cx: &mut Context<EditorView>,
  ) {
    match event {
      TestsContainerEvent::TestRenamed(path, _, new_name) => {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path == *path) {
          tab.name = new_name.clone();
          cx.notify();
        }
      }
      TestsContainerEvent::TestRemoved(path) => {
        let indexes_to_close: Vec<usize> = self
          .tabs
          .iter()
          .enumerate()
          .filter_map(|(ix, tab)| if path.is_parent(&tab.path) { Some(ix) } else { None })
          .collect();
        // Close from the end, as closing a tab shifts the indexes of the following ones
        indexes_to_close
          .into_iter()
          .rev()
          .for_each(|ix| self.close_tab(ix, window, cx));
      }
      TestsContainerEvent::RequestOpenInEditor(path) => {
        let active_ix = self.tabs.iter().position(|tab| tab.path == *path).or_else(|| {
          // If not already opened, fetch tab data, push in the list and return last ix
          if let Some(new_tab) = tests.read_with(cx, |tests, _| Self::tab_from_path(tests, path)) {
            self.tabs.push(new_tab);
            Some(self.tabs.len() - 1)
          } else {
            warn!("Requested opening an unknown path in editor: {}", path);
            self.active_tab.as_ref().map(|tab| tab.index)
          }
        });

        if active_ix != self.active_tab.as_ref().map(|active_tab| active_tab.index) {
          // Register the tab first, so the active index saved by `select_tab` is in bounds
          tests.update(cx, |tests, cx| {
            tests.open_test_in_editor(*path, active_ix, cx);
          });
          self.select_tab(active_ix, window, cx);
        }
        cx.notify();
      }
      TestsContainerEvent::TestMoved(old, new) => {
        if let Some(ix) = self.tabs.iter().position(|tab| tab.path == *old)
          && let Some(new_tab) = tests.read_with(cx, |tests, _| Self::tab_from_path(tests, new))
        {
          self.tabs[ix] = new_tab;
          // Rebuilds the editor if it was the active tab, as it is bound to the old path
          if self.active_tab.as_ref().is_some_and(|active_tab| active_tab.index == ix) {
            self.select_tab(Some(ix), window, cx);
          }
          cx.notify();
        }
      }
      _ => {}
    }
  }

  /// Makes the tab at `ix` the active one. Its editor is only rebuilt when the tab holds a
  /// different node than the current editor, so it is kept when the tab merely shifted.
  fn select_tab(&mut self, ix: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
    match (ix, self.active_tab.as_mut()) {
      (None, _) => self.active_tab = None,
      (Some(ix), Some(active_tab)) if active_tab.path == self.tabs[ix].path => active_tab.index = ix,
      (Some(ix), _) => self.active_tab = Some(ActiveTab::new(ix, &self.tabs[ix], &self.tests, window, cx)),
    }
    self.tests.update(cx, |tests, cx| tests.set_active_editor_tab(ix, cx));
    cx.notify();
  }

  fn tab_from_case_path(path: &TestPath, tests: &TestsContainer) -> Option<TabData> {
    tests.case_from_path(path).map(|case| TabData {
      path: *path,
      name: case.meta.name.clone(),
      kind: if case.is_case_step() {
        ProjectTestNodeKind::CaseStep
      } else {
        ProjectTestNodeKind::Case
      },
    })
  }

  /// Builds the tab of the node at `path`, or `None` if it doesn't exist in `tests`.
  fn tab_from_path(tests: &TestsContainer, path: &TestPath) -> Option<TabData> {
    match path {
      TestPath::Suite(_) => Self::tab_from_suite_path(path, tests),
      TestPath::Case(_, _) => Self::tab_from_case_path(path, tests),
      TestPath::Step(_, _, _) => Self::tab_from_step_path(path, tests),
    }
  }

  fn tab_from_suite_path(path: &TestPath, tests: &TestsContainer) -> Option<TabData> {
    tests.suite_from_path(path).map(|suite| TabData {
      path: *path,
      name: suite.meta.name.clone(),
      kind: ProjectTestNodeKind::Suite,
    })
  }

  fn tab_from_step_path(path: &TestPath, tests: &TestsContainer) -> Option<TabData> {
    tests.step_from_path(path).map(|step| TabData {
      path: *path,
      name: step.meta.name.clone(),
      kind: ProjectTestNodeKind::Step,
    })
  }
}

impl Render for EditorView {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
      .size_full()
      .child(
        h_flex()
          .w_full()
          .p_2()
          .h_12()
          .flex_row_reverse()
          .justify_between()
          .child(self.profile_selector.clone())
          .when_some(self.active_tab.as_ref(), |this, tab| {
            let selected_index = tab.index;
            this.child(
              TabBar::new("editor-tabs")
                .selected_index(selected_index)
                .on_click(cx.listener(|this, ix, window, cx| this.select_tab(Some(*ix), window, cx)))
                .children(self.tabs.iter().enumerate().map(|(ix, tab)| {
                  let icon = tab.kind.icon();
                  Tab::new()
                    .px_2()
                    .prefix(Icon::new(icon))
                    .suffix(
                      Button::new(format!("tab-close-{ix}"))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Close)
                        .on_click(cx.listener(move |this, _, window, cx| {
                          cx.stop_propagation();
                          this.close_tab(ix, window, cx);
                        })),
                    )
                    .label(tab.name.clone())
                })),
            )
          }),
      )
      .child(div().size_full().map(|this| {
        match self.active_tab.as_ref().map(|tab| &tab.editor) {
          Some(ActiveEditor::Suite(editor)) => this.child(editor.clone()),
          Some(ActiveEditor::Case(editor)) => this.child(editor.clone()),
          Some(ActiveEditor::Wip) => this.child(Empty::new().header(EmptyHeader::new().title(EmptyTitle::new().child("WIP")))),
          None => this.child(
            Empty::new().header(
              EmptyHeader::new()
                .title(EmptyTitle::new().child("No Tests opened"))
                .description(EmptyDescription::new().child("Open a test from the project tree.")),
            ),
          ),
        }
      }))
  }
}
