use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::{App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div};
use ki_workspace::TestCase;
use ki_workspace::test::TestMetadataOwner;
use crate::metadata_editor::MetadataEditor;

/// Editor of a test case.
pub struct CaseEditor {
  case: Entity<TestCase>,
  metadata_editor: Entity<MetadataEditor<TestCase>>,
}

impl CaseEditor {
  /// Creates the editor of `case`.
  pub fn new(case: Entity<TestCase>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
    let header_editor = cx.new(|_| MetadataEditor::new(case.clone()));
    Self { case, metadata_editor: header_editor }
  }

  /// Returns the label of the editor's tab: the current name of the test case.
  pub fn label(&self, cx: &App) -> SharedString {
    self.case.read(cx).name()
  }

  /// Returns the icon of the editor's tab.
  pub fn icon(&self, cx: &App) -> Icon {
    self.case.read(cx).icon(cx)
  }
}

impl Render for CaseEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .child(self.metadata_editor.clone())
      .child(Separator::horizontal())
  }
}
