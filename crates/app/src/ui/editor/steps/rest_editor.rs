use gpui_kit::base::v_flex;
use gpui_kit::component::label::Label;
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window};
use ki_workspace::test::StepDataProvider;

pub struct RestEditor<S: StepDataProvider> {
  step: Entity<S>,
}

impl<S: StepDataProvider> RestEditor<S> {
  pub(super) fn new(step: Entity<S>) -> Self {
    Self{
      step
    }
  }
}

impl<S: StepDataProvider> Render for RestEditor<S> {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<RestEditor<S>>) -> impl IntoElement {
    let _data = self.step.read(cx).data().unwrap();
    v_flex().size_full()
      .child(Label::new("Rest").text_lg())
  }
}
