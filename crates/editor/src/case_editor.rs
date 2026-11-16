use crate::metadata_editor::MetadataEditor;
use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::{
  App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
  Window, div,
};
use ki_app_shared::actions::{Redo, Undo};
use ki_utils::undo::UndoEntity;
use ki_workspace::TestCase;
use ki_workspace::test::TestMetadataOwner;

/// Editor of a test case.
pub struct CaseEditor {
  case: Entity<TestCase>,
  metadata_editor: Entity<MetadataEditor<TestCase>>,
  focus_handle: FocusHandle,
}

impl CaseEditor {
  fn on_action_undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
    self.case.update(cx, |case, cx| case.undo(cx));
  }

  fn on_action_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
    self.case.update(cx, |case, cx| case.redo(cx));
  }

  /// Creates the editor of `case`.
  pub fn new(case: Entity<TestCase>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
    let header_editor = cx.new(|_| MetadataEditor::new(case.clone()));
    Self {
      case,
      metadata_editor: header_editor,
      focus_handle: cx.focus_handle(),
    }
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
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .track_focus(&self.focus_handle)
      .on_action(cx.listener(Self::on_action_undo))
      .on_action(cx.listener(Self::on_action_redo))
      .size_full()
      .child(self.metadata_editor.clone())
      .child(Separator::horizontal())
  }
}
