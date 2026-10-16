use crate::ui::editor::header_editor::HeaderEditor;
use crate::ui::editor::steps::delay_editor::{DelayEditor, DelayEditorEvent};
use crate::ui::editor::steps::rest_editor::{RestEditor, RestEditorEvent};
use gpui_kit::component::Icon;
use gpui_kit::component::separator::Separator;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
  App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div,
};
use ki_workspace::TestStep;
use ki_workspace::test::TestMetadataOwner;
use ki_workspace::test::step_delay::DelayData;
use ki_workspace::test::test_step::StepData;

#[derive(Clone, Debug)]
pub enum StepDataEditor {
  Delay(Entity<DelayEditor>),
  Rest(Entity<RestEditor>),
}

/// Editor of a test step.
pub struct StepEditor {
  step: Entity<TestStep>,
  icon: Icon,
  data_editor: StepDataEditor,
  _data_editor_sub: Subscription,
}

impl StepEditor {
  /// Creates the editor of `step`.
  pub fn new(step: Entity<TestStep>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let (data_editor, _data_editor_sub) = Self::data_editor(&step, window, cx);
    Self {
      step: step.clone(),
      icon: step.read(cx).icon(),
      data_editor,
      _data_editor_sub,
    }
  }

  fn data_editor(step: &Entity<TestStep>, window: &mut Window, cx: &mut Context<Self>) -> (StepDataEditor, Subscription) {
    match step.read(cx).data().clone() {
      StepData::Rest(rest) => {
        let editor = cx.new(|cx| RestEditor::new(rest, window, cx));
        let _sub = cx.subscribe(&editor, |this, _, e, cx| Self::handle_rest_editor_event(this, e, cx));
        (StepDataEditor::Rest(editor), _sub)
      }
      StepData::Delay(delay) => {
        let editor = cx.new(|cx| DelayEditor::new(delay, window, cx));
        let _sub = cx.subscribe(&editor, |this, _, e, cx| Self::handle_delay_editor_event(this, e, cx));
        (StepDataEditor::Delay(editor), _sub)
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

  fn handle_delay_editor_event(&mut self, event: &DelayEditorEvent, cx: &mut Context<StepEditor>) {
    match event {
      DelayEditorEvent::DelayChanged(duration, unit) => {
        let new = DelayData {
          duration: *duration,
          unit: *unit,
        };
        self.step.update(cx, |step, cx| {
          step.update_data(cx, |data| match data {
            StepData::Delay(delay) if *delay != new => {
              *delay = new;
              true
            }
            _ => false,
          })
        })
      }
    }
  }

  fn handle_rest_editor_event(&mut self, _event: &RestEditorEvent, _cx: &mut Context<StepEditor>) {
    // TODO
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
