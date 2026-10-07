use crate::ui::editor::header_editor::HeaderEditor;
use gpui_kit::component::separator::Separator;
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};
use gpui_kit::component::empty::Empty;
use gpui_kit::prelude::FluentBuilder;
use ki_utils::TestPath;
use ki_workspace::test::TestsContainer;

/// Editor of a test case.
pub struct CaseEditor {
  tests: Entity<TestsContainer>,
  path: TestPath,
}

impl CaseEditor {
  /// Creates the editor of the case at `path`, held in `tests`.
  pub fn new(tests: Entity<TestsContainer>, path: TestPath, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
    Self { tests, path }
  }
}

impl Render for CaseEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let case = self.tests.read(cx).case_from_path(&self.path);
    div()
      .size_full()
      .when_some(case, |this, case| {
        let meta = case.meta.clone();
        this
          .child(HeaderEditor::new(meta, self.tests.clone()))
          .child(Separator::horizontal())
      })
      .when_none(&case, |this| {
        this.child(Empty::new())
      })
  }
}
