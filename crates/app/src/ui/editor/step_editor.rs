use crate::ui::editor::header_editor::HeaderEditor;
use crate::ui::editor::steps::StepDataEditor;
use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{App, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div};
use ki_workspace::TestStep;
use ki_workspace::test::TestMetadataOwner;

/// Editor of a test step.
pub struct StepEditor {
  step: Entity<TestStep>,
  data_editor: StepDataEditor,
}

impl StepEditor {
  /// Creates the editor of `step`.
  pub fn new(step: Entity<TestStep>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
    Self {
      step: step.clone(),
      data_editor: StepDataEditor::new(step, cx),
    }
  }

  /// Returns the label of the editor's tab: the current name of the test step.
  pub fn label(&self, cx: &App) -> SharedString {
    self.step.read(cx).name()
  }

  /// Returns the icon of the editor's tab.
  pub fn icon(&self, cx: &App) -> impl Into<Icon> {
    self.step.read(cx).icon()
  }
}

impl Render for StepEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .child(HeaderEditor::new(self.step.clone()))
      .child(Separator::horizontal())
      .child(div().size_full().p_2().map(|this| match &self.data_editor {
        StepDataEditor::Delay(editor) => this.child(editor.clone()),
        StepDataEditor::Rest(editor) => this.child(editor.clone()),
      }))
  }
}
