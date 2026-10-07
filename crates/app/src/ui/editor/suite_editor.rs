use crate::ui::editor::header_editor::HeaderEditor;
use gpui_kit::component::separator::Separator;
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};
use gpui_kit::component::empty::Empty;
use gpui_kit::prelude::FluentBuilder;
use ki_utils::TestPath;
use ki_workspace::test::TestsContainer;

/// Editor of a test suite.
pub struct SuiteEditor {
  tests: Entity<TestsContainer>,
  path: TestPath,
}

impl SuiteEditor {
  /// Creates the editor of the suite at `path`, held in `tests`.
  pub fn new(tests: Entity<TestsContainer>, path: TestPath, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
    Self { tests, path }
  }
}

impl Render for SuiteEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let suite = self.tests.read(cx).suite_from_path(&self.path);
    div()
      .size_full()
      .when_some(suite, |this, suite| {
        let meta = suite.meta.clone();
        this
          .child(HeaderEditor::new(meta, self.tests.clone()))
          .child(Separator::horizontal())
      })
      .when_none(&suite, |this| {
        this.child(Empty::new())
      })
  }
}
