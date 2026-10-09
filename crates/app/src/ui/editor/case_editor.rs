use crate::ui::editor::header_editor::HeaderEditor;
use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::{App, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div};
use ki_assets::icon::IconAsset;
use ki_workspace::TestCase;
use ki_workspace::test::TestMetadataOwner;

/// Editor of a test case.
pub struct CaseEditor {
  case: Entity<TestCase>,
}

impl CaseEditor {
  /// Creates the editor of `case`.
  pub fn new(case: Entity<TestCase>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
    Self { case }
  }

  /// Returns the label of the editor's tab: the current name of the test case.
  pub fn label(&self, cx: &App) -> SharedString {
    self.case.read(cx).name()
  }

  /// Returns the icon of the editor's tab, telling the case's kind (a case step shows as a step).
  pub fn icon(&self, cx: &App) -> impl Into<Icon> {
    if self.case.read(cx).is_case_step() {
      IconAsset::TestStep
    } else {
      IconAsset::TestCase
    }
  }
}

impl Render for CaseEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .child(HeaderEditor::new(self.case.clone()))
      .child(Separator::horizontal())
  }
}
