use gpui_kit::base::v_flex;
use gpui_kit::component::label::Label;
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window};
use ki_workspace::test::StepDataProvider;

pub struct DelayEditor<S: StepDataProvider> {
  step: Entity<S>,
}

impl<S: StepDataProvider> DelayEditor<S> {
  pub(super) fn new(step: Entity<S>) -> Self {
    Self{
      step
    }
  }
}

impl<S: StepDataProvider> Render for DelayEditor<S> {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<DelayEditor<S>>) -> impl IntoElement {
    let _data = self.step.read(cx).data().unwrap();
    v_flex().size_full()
      .child(Label::new("Delay").text_lg())
  }
}
