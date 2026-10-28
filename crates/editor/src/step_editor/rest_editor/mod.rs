mod body_editor;
mod param_editor;

use crate::step_editor::StepDataEvent;
use crate::step_editor::rest_editor::body_editor::RestBodyEditor;
use crate::step_editor::rest_editor::param_editor::RestParamEditor;
use gpui_kit::base::input::InputEvent;
use gpui_kit::base::{IndexPath, h_flex, v_flex};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{ActiveTheme, Colorize, Icon, IconName};
use gpui_kit::{
  AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement, Render, Styled, Subscription,
  Window, div,
};
use ki_app_shared::panel::{Panel, PanelContainer};
use ki_utils::http::HttpMethod;
use ki_workspace::test::step_rest::RestData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestEditorPanel {
  Params,
  Body,
}

pub struct RestEditor {
  rest_data: Entity<RestData>,
  endpoint_input_state: Entity<InputState>,
  method_select_state: Entity<SelectState<Vec<HttpMethod>>>,
  panel_container: PanelContainer<RestEditorPanel>,
  _subs: Vec<Subscription>,
}

impl RestEditor {
  pub fn new(rest_data: Entity<RestData>, window: &mut Window, cx: &mut Context<RestEditor>) -> Self {
    let url_input_state = cx.new(|cx| InputState::new(window, cx).default_value(rest_data.read(cx).endpoint().clone()));
    let init_method_ix = HttpMethod::ALL
      .iter()
      .position(|method| *method == rest_data.read(cx).method())
      .map(IndexPath::new);
    let method_select_state = cx.new(|cx| SelectState::new(HttpMethod::ALL.to_vec(), init_method_ix, window, cx));
    let _subs = vec![
      cx.subscribe(&url_input_state, |this, _, e: &InputEvent, cx| match e {
        InputEvent::PressEnter { .. } | InputEvent::Blur => this.handle_endpoint_select_event(cx),
        _ => {}
      }),
      cx.subscribe(&method_select_state, |this, _, _: &SelectEvent<Vec<HttpMethod>>, cx| {
        this.handle_method_select_event(cx)
      }),
    ];

    let panel_container = PanelContainer::new(
      vec![
        Panel::build::<RestData, RestParamEditor>(rest_data.clone(), window, cx),
        Panel::build::<RestData, RestBodyEditor>(rest_data.clone(), window, cx),
      ],
      RestEditorPanel::Params,
    );

    Self {
      rest_data,
      endpoint_input_state: url_input_state,
      method_select_state,
      panel_container,
      _subs,
    }
  }

  fn handle_method_select_event(&mut self, cx: &mut Context<Self>) {
    let method = *self.method_select_state.read(cx).selected_value().unwrap();
    if self.rest_data.update(cx, move |rest, cx| rest.set_method(method, cx)) {
      cx.emit(StepDataEvent::IconChanged);
    }
  }

  fn handle_endpoint_select_event(&mut self, cx: &mut Context<Self>) {
    let endpoint = self.endpoint_input_state.read(cx).selected_value().clone();
    self.rest_data.update(cx, move |rest, cx| rest.set_endpoint(endpoint, cx));
  }
}

impl EventEmitter<StepDataEvent> for RestEditor {}

impl Render for RestEditor {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<RestEditor>) -> impl IntoElement {
    v_flex()
      .size_full()
      .child(
        h_flex()
          .w_full()
          .p_2()
          .gap_x_2()
          .child(
            h_flex()
              .gap_x_0()
              .flex_grow_1()
              .child(
                div().child(
                  Select::new(&self.method_select_state)
                    .text_color(self.rest_data.read(cx).method().color())
                    .border_r_0()
                    .w_32()
                    .rounded_r_none(),
                ),
              )
              .child(
                Input::new(&self.endpoint_input_state)
                  .border_l_0()
                  .rounded_l_none()
                  .flex_grow_1(),
              ),
          )
          .child(
            div()
              .id("run-rest_editor-step")
              .flex()
              .flex_shrink_0()
              .items_center()
              .justify_center()
              .size_8()
              .rounded(cx.theme().radius)
              .border_1()
              .border_color(cx.theme().input)
              .text_color(gpui_kit::green())
              .hover(|this| this.border_color(gpui_kit::green().darken(0.3)))
              .child(Icon::new(IconName::Play)),
          ),
      )
      .child(
        TabBar::new("tabs")
          .selected_index(self.panel_container.selected_ix())
          .segmented()
          .on_click(cx.listener(|this, selected_index: &usize, _, cx| {
            this.panel_container.set_selected_ix(*selected_index);
            cx.notify();
          }))
          .children(self.panel_container.tabs()),
      )
      .child(self.panel_container.view())
  }
}
