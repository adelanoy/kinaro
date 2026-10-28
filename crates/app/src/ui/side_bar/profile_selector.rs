use crate::ui::components::profile_select;
use gpui_kit::base::h_flex;
use gpui_kit::component::Sizable;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::{App, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled, Subscription, Window};
use ki_workspace::variable::{Profile, ProjectVariables, ProjectVariablesEvent};

pub struct ProfileSelector {
  focus_handle: FocusHandle,
  _vars_event_sub: Subscription,
  profile_select_state: Entity<SelectState<Vec<Profile>>>,
}

impl ProfileSelector {
  pub fn new(vars: Entity<ProjectVariables>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let profile_select_state = profile_select(&vars, window, cx);
    cx.subscribe(&profile_select_state, {
      let vars = vars.clone();
      move |_, _, event: &SelectEvent<Vec<Profile>>, cx| match event {
        SelectEvent::Confirm(value) => vars.update(cx, |this, cx| this.switch_profile(*value, cx)),
      }
    })
    .detach();

    let _vars_event_sub = cx.subscribe_in(&vars, window, move |this, vars, event, window, cx| match event {
      ProjectVariablesEvent::ProfilesChanged => {
        this
          .profile_select_state
          .update(cx, |state, cx| state.set_items(vars.read(cx).profiles.clone(), window, cx));
      }
      ProjectVariablesEvent::ActiveProfile(id) => match id {
        None => this
          .profile_select_state
          .update(cx, |state, cx| state.set_selected_index(None, window, cx)),
        Some(id) => {
          this
            .profile_select_state
            .update(cx, |state, cx| state.set_selected_value(id, window, cx));
        }
      },
      _ => {}
    });

    Self {
      focus_handle: cx.focus_handle(),
      _vars_event_sub,
      profile_select_state,
    }
  }
}

impl Focusable for ProfileSelector {
  fn focus_handle(&self, _cx: &App) -> FocusHandle {
    self.focus_handle.clone()
  }
}

impl Render for ProfileSelector {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    h_flex()
      .text_sm()
      .gap_x_2()
      .child("Active Profile")
      .child(Select::new(&self.profile_select_state).small())
  }
}
