use crate::ui::editor::header_editor::HeaderEditor;
use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::{App, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div};
use ki_assets::icon::IconAsset;
use ki_workspace::TestStep;
use ki_workspace::test::TestMetadataOwner;

/// Editor of a test step.
pub struct StepEditor {
  step: Entity<TestStep>,
}

impl StepEditor {
  /// Creates the editor of `step`.
  pub fn new(step: Entity<TestStep>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
    Self { step }
  }

  /// Returns the label of the editor's tab: the current name of the test step.
  pub fn label(&self, cx: &App) -> SharedString {
    self.step.read(cx).name()
  }

  /// Returns the icon of the editor's tab.
  pub fn icon(&self, _cx: &App) -> impl Into<Icon> {
    IconAsset::TestStep
  }
}

impl Render for StepEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .child(HeaderEditor::new(self.step.clone()))
      .child(Separator::horizontal())
  }
}
