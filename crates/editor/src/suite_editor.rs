use crate::metadata_editor::MetadataEditor;
use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::{App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div};
use ki_workspace::TestSuite;
use ki_workspace::test::TestMetadataOwner;

/// Editor of a test suite.
pub struct SuiteEditor {
  suite: Entity<TestSuite>,
  metadata_editor: Entity<MetadataEditor<TestSuite>>,
}

impl SuiteEditor {
  /// Creates the editor of `suite`.
  pub fn new(suite: Entity<TestSuite>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
    let header_editor = cx.new(|_| MetadataEditor::new(suite.clone()));
    Self {
      suite,
      metadata_editor: header_editor,
    }
  }

  /// Returns the label of the editor's tab: the current name of the test suite.
  pub fn label(&self, cx: &App) -> SharedString {
    self.suite.read(cx).name()
  }

  /// Returns the icon of the editor's tab.
  pub fn icon(&self, cx: &App) -> Icon {
    self.suite.read(cx).icon(cx)
  }
}

impl Render for SuiteEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .child(self.metadata_editor.clone())
      .child(Separator::horizontal())
  }
}
