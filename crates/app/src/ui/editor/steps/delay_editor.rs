use gpui_kit::base::v_flex;
use gpui_kit::component::label::Label;
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window};
use ki_workspace::TestStep;

pub struct DelayEditor {
  step: Entity<TestStep>,
}

impl DelayEditor {
  pub(super) fn new(step: Entity<TestStep>) -> Self {
    Self{
      step
    }
  }
}

impl Render for DelayEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<DelayEditor>) -> impl IntoElement {
    let _data = self.step.read(cx).data();
    v_flex().size_full()
      .child(Label::new("Delay").text_lg())
  }
}
