mod profile_selector;

use crate::ui::ProjectTestNodeKind;
use crate::ui::editor::profile_selector::ProfileSelector;
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

/// Display data of a tab opened in the editor
struct TabData {
  path: TestPath,
  name: SharedString,
  kind: ProjectTestNodeKind,
}

/// The main editor view: one tab per test node opened from the project tree.
///
/// The opened tabs and the active one are mirrored in the project's [`TestsContainer`] so they
/// are persisted in the workspace, and kept in sync with the test tree (renames, moves, removals).
pub struct Editor {
  tests: Entity<TestsContainer>,
  tabs: Vec<TabData>,
  profile_selector: Entity<ProfileSelector>,
  active_tab_ix: Option<usize>,
  _tests_sub: Subscription,
}

impl Editor {
  /// Closes the tab at `closed_ix`, selecting the next tab (or the previous one when it was the
  /// last) if it was the active one.
  fn close_tab(&mut self, closed_ix: usize, cx: &mut Context<Self>) {
    let tab = self.tabs.remove(closed_ix);
    // Check if the active tab changed
    let new_active_tab_ix = if self.tabs.is_empty() {
      None
    } else if Some(closed_ix) == self.active_tab_ix {
      if closed_ix < self.tabs.len() {
        self.active_tab_ix
      } else {
        Some(closed_ix - 1)
      }
    } else if let Some(current_ix) = self.active_tab_ix
      && current_ix > closed_ix
    {
      Some(current_ix - 1)
    } else {
      self.active_tab_ix
    };

    let index_updated = if new_active_tab_ix != self.active_tab_ix {
      self.active_tab_ix = new_active_tab_ix;
      true
    } else {
      false
    };
    self.tests.update(cx, |tests, cx| {
      tests.close_test_from_editor(tab.path, cx);
      if index_updated {
        tests.set_active_editor_tab(new_active_tab_ix, cx);
      }
    });
    cx.notify();
  }

  /// Builds the tabs from the ones saved in `tests`, making sure there is an active tab index if
  /// and only if there are tabs.
  fn get_tabs(tests: &Entity<TestsContainer>, cx: &mut Context<Self>) -> (Vec<TabData>, Option<usize>) {
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
    if !tabs.is_empty() && ix.is_none() {
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
    let _tests_sub = cx.subscribe(&tests, Self::tests_event_listener);
    let (tabs, active_tab_ix) = Self::get_tabs(&tests, cx);
    let vars = project.read(cx).variables.clone();
    let profile_selector = cx.new(|cx| ProfileSelector::new(vars, window, cx));
    Self {
      tests,
      tabs,
      profile_selector,
      active_tab_ix,
      _tests_sub,
    }
  }

  /// Keeps the tabs in sync with the test tree: opens (or selects) a tab on open requests, and
  /// updates the tabs of renamed, moved or removed nodes.
  fn tests_event_listener(&mut self, tests: Entity<TestsContainer>, event: &TestsContainerEvent, cx: &mut Context<Editor>) {
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
        indexes_to_close.into_iter().rev().for_each(|ix| self.close_tab(ix, cx));
      }
      TestsContainerEvent::RequestOpenInEditor(path) => {
        if let Some(new_tab) = tests.read_with(cx, |tests, _| Self::tab_from_path(tests, path)) {
          let path = new_tab.path;
          let active_tab_ix = match self.tabs.iter().position(|tab| tab.path == path) {
            None => {
              self.tabs.push(new_tab);
              Some(self.tabs.len() - 1)
            }
            Some(pos) => Some(pos),
          };
          self.active_tab_ix = active_tab_ix;
          tests.update(cx, |tests, cx| {
            tests.open_test_in_editor(path, self.active_tab_ix, cx);
          });
          cx.notify();
        }
      }
      TestsContainerEvent::TestMoved(old, new) => {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path == *old)
          && let Some(new_tab) = tests.read_with(cx, |tests, _| Self::tab_from_path(tests, new))
        {
          *tab = new_tab;
          cx.notify();
        }
      }
      _ => {}
    }
  }

  /// Makes the tab at `ix` the active one.
  fn select_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
    self.active_tab_ix = Some(ix);
    self
      .tests
      .update(cx, |tests, cx| tests.set_active_editor_tab(self.active_tab_ix, cx));
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

impl Render for Editor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let tabs = &self.tabs;
    v_flex()
      .p_2()
      .size_full()
      .child(
        h_flex()
          .w_full()
          .h_12()
          .flex_row_reverse()
          .justify_between()
          .child(self.profile_selector.clone())
          .when(!tabs.is_empty(), |this| {
            let selected_index = self.active_tab_ix.unwrap();
            this.child(
              TabBar::new("editor-tabs")
                .selected_index(selected_index)
                .on_click(cx.listener(|this, ix, _, cx| this.select_tab(*ix, cx)))
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
                        .on_click(cx.listener(move |this, _, _, cx| {
                          this.close_tab(ix, cx);
                        })),
                    )
                    .label(tab.name.clone())
                })),
            )
          }),
      )
      .child(div().size_full().when_else(
        tabs.is_empty(),
        |this| {
          this.child(
            Empty::new().header(
              EmptyHeader::new()
                .title(EmptyTitle::new().child("No Tests opened"))
                .description(EmptyDescription::new().child("Open a test from the project tree.")),
            ),
          )
        },
        |this| this.child(Empty::new().header(EmptyHeader::new().title(EmptyTitle::new().child("WIP")))),
      ))
  }
}
