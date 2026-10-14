use gpui_kit::base::v_flex;
use gpui_kit::component::label::Label;
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window};
use ki_workspace::TestStep;

pub struct RestEditor {
  step: Entity<TestStep>,
}

impl RestEditor {
  pub(super) fn new(step: Entity<TestStep>) -> Self {
    Self{
      step
    }
  }
}

impl Render for RestEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<RestEditor>) -> impl IntoElement {
    let _data = self.step.read(cx).data();
    v_flex().size_full()
      .child(Label::new("Rest").text_lg())
  }
}
