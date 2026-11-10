use gpui_kit::base::input::InputEvent;
use gpui_kit::base::{IndexPath, h_flex, v_flex};
use gpui_kit::component::input::{InputState, NumberInput};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::{AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window, div};
use ki_utils::time::TimeUnit;
use ki_workspace::test::step_delay::{DelayData, DelayDataEvent};
use log::warn;

pub struct DelayEditor {
  delay: Entity<DelayData>,
  duration_input_state: Entity<InputState>,
  unit_select_state: Entity<SelectState<Vec<TimeUnit>>>,
  _subs: Vec<Subscription>,
}

impl DelayEditor {
  pub fn new(delay: Entity<DelayData>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let unit_select_state = cx.new(|cx| SelectState::new(TimeUnit::ALL.to_vec(), Some(IndexPath::default()), window, cx));
    let duration_input_state = cx.new(|cx| InputState::new(window, cx));

    let _subs = vec![
      cx.subscribe(&unit_select_state, |this, _, _: &SelectEvent<Vec<TimeUnit>>, cx| {
        this.input_changed(cx);
      }),
      cx.subscribe(&duration_input_state, |this, _, e: &InputEvent, cx| {
        if matches!(e, InputEvent::Blur | InputEvent::PressEnter { secondary: _, shift: _ }) {
          this.input_changed(cx);
        }
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
    let Ok(duration) = self.duration_input_state.read(cx).value().as_str().parse::<u32>() else {
      warn!("Invalid duration input: {}", self.duration_input_state.read(cx).value());
      return;
    };
    let unit = *self.unit_select_state.read(cx).selected_value().unwrap();
    self.delay.update(cx, |delay, cx| delay.update(duration, unit, cx));
  }

  fn update_states(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let unit = self.delay.read(cx).unit();
    self.unit_select_state.update(cx, |state, cx| {
      state.set_selected_value(&unit, window, cx);
    });
    let duration = self.delay.read(cx).duration();
    self
      .duration_input_state
      .update(cx, |this, cx| this.set_value(duration.to_string(), window, cx));
  }
}

impl Render for DelayEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<DelayEditor>) -> impl IntoElement {
    v_flex().size_full().child("Wait for:").child(
      h_flex().gap_x_2().child("Duration: ").child(
        div()
          .flex_1()
          .child(NumberInput::new(&self.duration_input_state).suffix(Select::new(&self.unit_select_state).min_w_32())),
      ),
    )
  }
}
