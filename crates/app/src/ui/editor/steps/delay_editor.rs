use gpui_kit::base::input::InputEvent;
use gpui_kit::base::{IndexPath, h_flex, v_flex};
use gpui_kit::component::input::{InputState, NumberInput};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::{
  AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled, Subscription, Window, div,
};
use ki_utils::time::TimeUnit;
use ki_workspace::test::step_delay::DelayData;
use log::warn;

pub enum DelayEditorEvent {
  DelayChanged(u32, TimeUnit),
}

pub struct DelayEditor {
  duration_input_state: Entity<InputState>,
  unit_select_state: Entity<SelectState<Vec<TimeUnit>>>,
  _subs: Vec<Subscription>,
}

impl DelayEditor {
  pub fn new(delay: DelayData, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let unit_pos = TimeUnit::ALL.iter().position(|tu| *tu == delay.unit);
    let unit_select_state = cx.new(|cx| SelectState::new(TimeUnit::ALL.to_vec(), unit_pos.map(IndexPath::new), window, cx));
    let duration_input_state = cx.new(|cx| InputState::new(window, cx).default_value(delay.duration.to_string()));

    let _subs = vec![
      cx.subscribe(&unit_select_state, |this, _, _: &SelectEvent<Vec<TimeUnit>>, cx| {
        Self::input_changed(this, cx);
      }),
      cx.subscribe(&duration_input_state, |this, _, e: &InputEvent, cx| {
        if matches!(e, InputEvent::Blur | InputEvent::PressEnter { secondary: _, shift: _ }) {
          Self::input_changed(this, cx);
        }
      }),
    ];

    Self {
      duration_input_state,
      unit_select_state,
      _subs,
    }
  }

  fn input_changed(&mut self, cx: &mut Context<Self>) {
    let Ok(duration) = self.duration_input_state.read(cx).value().as_str().parse::<u32>() else {
      warn!("Invalid duration input: {}", self.duration_input_state.read(cx).value());
      return;
    };
    let unit = *self.unit_select_state.read(cx).selected_value().unwrap();
    cx.emit(DelayEditorEvent::DelayChanged(duration, unit))
  }
}

impl EventEmitter<DelayEditorEvent> for DelayEditor {}

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
