use crate::metadata_editor::MetadataEditor;
use crate::step_editor::delay_editor::DelayEditor;
use crate::step_editor::rest_editor::RestEditor;
use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
  App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div,
};
use ki_workspace::TestStep;
use ki_workspace::test::TestMetadataOwner;
use ki_workspace::test::test_step::StepData;

mod delay_editor;
mod rest_editor;

enum StepDataEvent {
  IconChanged,
}

#[derive(Clone, Debug)]
pub enum StepDataEditor {
  Delay(Entity<DelayEditor>),
  Rest(Entity<RestEditor>),
}

/// Editor of a test step.
pub struct StepEditor {
  step: Entity<TestStep>,
  metadata_editor: Entity<MetadataEditor<TestStep>>,
  icon: Icon,
  data_editor: StepDataEditor,
  _data_editor_sub: Option<Subscription>,
}

impl StepEditor {
  /// Creates the editor of `step`.
  pub fn new(step: Entity<TestStep>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let header_editor = cx.new(|_| MetadataEditor::new(step.clone()));
    let (data_editor, _data_editor_sub) = Self::data_editor(&step, window, cx);
    Self {
      step: step.clone(),
      metadata_editor: header_editor,
      icon: step.read_with(cx, |step, cx| step.icon(cx)),
      data_editor,
      _data_editor_sub,
    }
  }

  fn data_editor(step: &Entity<TestStep>, window: &mut Window, cx: &mut Context<Self>) -> (StepDataEditor, Option<Subscription>) {
    match step.read(cx).data().clone() {
      StepData::Rest(rest) => {
        let editor = cx.new(|cx| RestEditor::new(rest, window, cx));
        let sub = cx.subscribe(&editor, |this, _, _: &StepDataEvent, cx| {
          this.icon = this.step.read_with(cx, |step, cx| step.icon(cx));
        });
        (StepDataEditor::Rest(editor), Some(sub))
      }
      StepData::Delay(delay) => {
        let editor = cx.new(|cx| DelayEditor::new(delay, window, cx));
        (StepDataEditor::Delay(editor), None)
      }
    }
  }

  /// Returns the label of the editor's tab: the current name of the test step.
  pub fn label(&self, cx: &App) -> SharedString {
    self.step.read(cx).name()
  }

  /// Returns the icon of the editor's tab.
  pub fn icon(&self, _cx: &App) -> Icon {
    self.icon.clone()
  }
}

impl Render for StepEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .child(self.metadata_editor.clone())
      .child(Separator::horizontal())
      .child(div().size_full().map(|this| match &self.data_editor {
        StepDataEditor::Delay(editor) => this.child(editor.clone()),
        StepDataEditor::Rest(editor) => this.child(editor.clone()),
      }))
  }
}
