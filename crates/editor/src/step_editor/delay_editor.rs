use gpui_kit::base::{IndexPath, h_flex, v_flex};
use gpui_kit::component::button::{Toggle, ToggleVariants};
use gpui_kit::component::input::{Input, NumberInput};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window, div};
use ki_app_shared::input::{ExprValueInputEvent, ExprValueInputState};
use ki_assets::icon::IconAsset;
use ki_utils::time::TimeUnit;
use ki_workspace::test::step_delay::{DelayData, DelayDataEvent};

pub struct DelayEditor {
  delay: Entity<DelayData>,
  duration_input_state: Entity<ExprValueInputState<u32>>,
  unit_select_state: Entity<SelectState<Vec<TimeUnit>>>,
  _subs: Vec<Subscription>,
}

impl DelayEditor {
  pub fn new(delay: Entity<DelayData>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let unit_select_state = cx.new(|cx| SelectState::new(TimeUnit::ALL.to_vec(), Some(IndexPath::default()), window, cx));
    let duration_input_state = cx.new(|cx| ExprValueInputState::new(delay.read(cx).duration(), 1000, "1000", window, cx));

    let _subs = vec![
      cx.subscribe(&unit_select_state, |this, _, _: &SelectEvent<Vec<TimeUnit>>, cx| {
        this.input_changed(cx);
      }),
      cx.subscribe(&duration_input_state, |this, _, _: &ExprValueInputEvent, cx| {
        this.input_changed(cx)
      }),
      cx.subscribe_in(&delay, window, |this, _, e: &DelayDataEvent, window, cx| {
        if matches!(e, DelayDataEvent::DataChanged) {
          this.update_states(window, cx);
        }
      }),
    ];
    let mut this = Self {
      delay,
      duration_input_state,
      unit_select_state,
      _subs,
    };
    this.update_states(window, cx);
    this
  }

  fn input_changed(&mut self, cx: &mut Context<Self>) {
    let duration = self.duration_input_state.read(cx).value();
    let unit = *self.unit_select_state.read(cx).selected_value().unwrap();
    self.delay.update(cx, |delay, cx| delay.update(duration, unit, cx));
  }

  fn update_states(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let (duration, unit) = self.delay.read_with(cx, |delay, _| (delay.duration(), delay.unit()));
    self.unit_select_state.update(cx, |state, cx| {
      state.set_selected_value(&unit, window, cx);
    });
    self
      .duration_input_state
      .update(cx, |state, cx| state.set_value(duration, window, cx));
  }
}

impl Render for DelayEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<DelayEditor>) -> impl IntoElement {
    let is_expr = self.duration_input_state.read(cx).is_expr();
    let duration_state = self.duration_input_state.read(cx).state();
    v_flex().w_full().child("Wait for:").child(
      h_flex()
        .w_full()
        .gap_x_2()
        .child("Duration: ")
        .child(
          h_flex()
            .flex_1()
            .child(
              Toggle::new("type-selection")
                .icon(if is_expr { IconAsset::Braces } else { IconAsset::Numbers })
                .rounded_l_md()
                .rounded_r_none()
                .outline()
                .checked(is_expr)
                .on_click(cx.listener(|this, value, window, cx| {
                  this
                    .duration_input_state
                    .update(cx, |state, cx| state.switch(*value, window, cx))
                })),
            )
            .child(div().flex_grow_1().when_else(
              is_expr,
              |this| this.child(Input::new(duration_state).rounded_r_md().rounded_l_none()),
              |this| this.child(NumberInput::new(duration_state).rounded_r_md().rounded_l_none()),
            )),
        )
        .child(div().flex_none().child(Select::new(&self.unit_select_state).w_48())),
    )
  }
}
