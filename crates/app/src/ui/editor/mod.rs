mod case_editor;
mod header_editor;
mod profile_selector;
mod step_editor;
mod suite_editor;

use crate::ui::editor::case_editor::CaseEditor;
use crate::ui::editor::profile_selector::ProfileSelector;
use crate::ui::editor::step_editor::StepEditor;
use crate::ui::editor::suite_editor::SuiteEditor;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Icon, IconName, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
  App, AppContext, Context, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString, Styled,
  Subscription, Window, div,
};
use ki_utils::TestPath;
use ki_workspace::Project;
use ki_workspace::test::{TestsContainer, TestsContainerEvent};
use log::warn;

/// A tab of the editor: the node it shows and its editor.
struct TabEditor {
  /// Path of the node shown in the tab
  path: TestPath,
  /// Editor of the node
  editor: TestEditor,
}

/// The editor shown for the active tab, chosen from the kind of its node.
enum TestEditor {
  Suite(Entity<SuiteEditor>),
  Case(Entity<CaseEditor>),
  Step(Entity<StepEditor>),
}

impl TestEditor {
  /// Returns the label of the tab: the current name of the edited node.
  fn label(&self, cx: &App) -> SharedString {
    match &self {
      TestEditor::Suite(suite_editor) => suite_editor.read(cx).label(cx),
      TestEditor::Case(case_editor) => case_editor.read(cx).label(cx),
      TestEditor::Step(step_editor) => step_editor.read(cx).label(cx),
    }
  }

  /// Returns the icon of the tab, telling the kind of the edited node.
  fn icon(&self, cx: &App) -> Icon {
    match &self {
      TestEditor::Suite(suite_editor) => Icon::new(suite_editor.read(cx).icon(cx)),
      TestEditor::Case(case_editor) => Icon::new(case_editor.read(cx).icon(cx)),
      TestEditor::Step(step_editor) => Icon::new(step_editor.read(cx).icon(cx)),
    }
  }
}

/// The main editor view: one tab per test node opened from the project tree.
///
/// The opened tabs and the active one are mirrored in the project's [`TestsContainer`] so they
/// are persisted in the workspace, and kept in sync with the test tree (moves, removals). Tab labels
/// are read from the edited nodes, so they follow renames.
pub struct EditorView {
  tests: Entity<TestsContainer>,
  tabs: Vec<TabEditor>,
  active_tab: Option<usize>,
  profile_selector: Entity<ProfileSelector>,
  _tests_sub: Subscription,
}

impl EditorView {
  /// Closes the tab at `closed_ix`, selecting the next tab (or the previous one when it was the
  /// last) if it was the active one.
  fn close_tab(&mut self, closed_ix: usize, _window: &mut Window, cx: &mut Context<Self>) {
    let tab = self.tabs.remove(closed_ix);
    self.tests.update(cx, |tests, cx| tests.close_test_from_editor(tab.path, cx));
    let new_active_tab_ix = match self.active_tab {
      _ if self.tabs.is_empty() => None,
      Some(current_ix) if current_ix > closed_ix => Some(current_ix - 1),
      Some(current_ix) => Some(current_ix.min(self.tabs.len() - 1)),
      None => None,
    };
    self.select_tab(new_active_tab_ix, cx);
  }

  /// Builds the tabs from the ones saved in `tests`, making sure there is an active tab index if
  /// and only if there are tabs.
  fn get_tabs(tests: &Entity<TestsContainer>, window: &mut Window, cx: &mut Context<Self>) -> (Vec<TabEditor>, Option<usize>) {
    let (opened_nodes, mut ix) = tests.read_with(cx, |tests, _| {
      (tests.opened_editor_nodes().to_vec(), tests.active_editor_tab_index())
    });
    let tabs = opened_nodes
      .iter()
      .filter_map(|path| Self::tab_from_path(tests, path, window, cx))
      .collect::<Vec<_>>();
    // Check coherency of index
    if !tabs.is_empty() && ix.is_none_or(|ix| ix >= tabs.len()) {
      ix = Some(0);
      tests.update(cx, |tests, cx| tests.set_active_editor_tab(ix, cx));
    }
    if tabs.is_empty() && ix.is_some() {
      ix = None;
      tests.update(cx, |tests, cx| tests.set_active_editor_tab(ix, cx));
    }
    (tabs, ix)
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

  /// Keeps the tabs in sync with the test tree: opens (or selects) a tab on open requests, rebuilds
  /// the tabs of moved nodes and closes the tabs of removed nodes and of their descendants.
  fn tests_event_listener(
    &mut self,
    tests: &Entity<TestsContainer>,
    event: &TestsContainerEvent,
    window: &mut Window,
    cx: &mut Context<EditorView>,
  ) {
    match event {
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
          if let Some(new_tab) = Self::tab_from_path(tests, path, window, cx) {
            self.tabs.push(new_tab);
            Some(self.tabs.len() - 1)
          } else {
            warn!("Requested opening an unknown path in editor: {}", path);
            self.active_tab
          }
        });

        if active_ix != self.active_tab {
          // Register the tab first, so the active index saved by `select_tab` is in bounds
          tests.update(cx, |tests, cx| {
            tests.open_test_in_editor(*path, active_ix, cx);
          });
          self.select_tab(active_ix, cx);
        }
        cx.notify();
      }
      TestsContainerEvent::TestMoved(old, new) => {
        if let Some(ix) = self.tabs.iter().position(|tab| tab.path == *old)
          && let Some(tab) = Self::tab_from_path(tests, new, window, cx)
        {
          self.tabs[ix] = tab;
          cx.notify();
        }
      }
      _ => {}
    }
  }

  /// Makes the tab at `ix` the active one (no tab with `None`), saving it in the tests.
  fn select_tab(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
    if ix != self.active_tab {
      self.active_tab = ix;
      self.tests.update(cx, |tests, cx| tests.set_active_editor_tab(ix, cx));
      cx.notify();
    }
  }

  /// Builds the tab of the case at `path`, or `None` if it doesn't exist in `tests`.
  fn tab_from_case_path(
    path: &TestPath,
    tests: &Entity<TestsContainer>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Option<TabEditor> {
    tests.read(cx).case_from_path(path, cx).map(|case| TabEditor {
      path: *path,
      editor: { TestEditor::Case(cx.new(|cx| CaseEditor::new(case, window, cx))) },
    })
  }

  /// Builds the tab of the node at `path`, or `None` if it doesn't exist in `tests`.
  fn tab_from_path(
    tests: &Entity<TestsContainer>,
    path: &TestPath,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Option<TabEditor> {
    match path {
      TestPath::Suite(_) => Self::tab_from_suite_path(path, tests, window, cx),
      TestPath::Case(_, _) => Self::tab_from_case_path(path, tests, window, cx),
      TestPath::Step(_, _, _) => Self::tab_from_step_path(path, tests, window, cx),
    }
  }

  /// Builds the tab of the suite at `path`, or `None` if it doesn't exist in `tests`.
  fn tab_from_suite_path(
    path: &TestPath,
    tests: &Entity<TestsContainer>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Option<TabEditor> {
    tests.read(cx).suite_from_path(path, cx).map(|suite| TabEditor {
      path: *path,
      editor: TestEditor::Suite(cx.new(|cx| SuiteEditor::new(suite, window, cx))),
    })
  }

  /// Builds the tab of the step at `path`, or `None` if it doesn't exist in `tests`.
  fn tab_from_step_path(
    path: &TestPath,
    tests: &Entity<TestsContainer>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Option<TabEditor> {
    tests.read(cx).step_from_path(path, cx).map(|step| TabEditor {
      path: *path,
      editor: TestEditor::Step(cx.new(|cx| StepEditor::new(step, window, cx))),
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
          .when_some(self.active_tab, |this, tab_ix| {
            this.child(
              TabBar::new("editor-tabs")
                .selected_index(tab_ix)
                .on_click(cx.listener(|this, ix, _, cx| this.select_tab(Some(*ix), cx)))
                .children(self.tabs.iter().enumerate().map(|(ix, tab)| {
                  Tab::new()
                    .px_2()
                    .on_mouse_down(
                      MouseButton::Middle,
                      cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.close_tab(ix, window, cx);
                      }),
                    )
                    .prefix(tab.editor.icon(cx))
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
                    .label(tab.editor.label(cx))
                })),
            )
          }),
      )
      .child(div().size_full().map(|this| {
        match self.active_tab.and_then(|tab| self.tabs.get(tab)).map(|tab| &tab.editor) {
          Some(TestEditor::Suite(editor)) => this.child(editor.clone()),
          Some(TestEditor::Case(editor)) => this.child(editor.clone()),
          Some(TestEditor::Step(editor)) => this.child(editor.clone()),
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
