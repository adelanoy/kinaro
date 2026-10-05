use gpui_kit::base::{h_flex, IndexPath};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::{App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled, Subscription, Window};
use gpui_kit::component::Sizable;
use ki_workspace::variable::{ProfileInfo, ProjectVariables, ProjectVariablesEvent};
use uuid::Uuid;

pub(super) struct ProfileSelector {
  focus_handle: FocusHandle,
  _vars_event_sub: Subscription,
  profile_select_state: Entity<SelectState<Vec<ProfileInfo>>>,
}

impl ProfileSelector {
  pub fn new(vars: Entity<ProjectVariables>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let profile_select_state = cx.new(|cx| SelectState::new(vec![], Some(IndexPath::default()), window, cx));
    cx.subscribe(&profile_select_state, {
      let vars = vars.clone();
      move |_, _, event: &SelectEvent<Vec<ProfileInfo>>, cx| match event {
        SelectEvent::Confirm(value) => vars.update(cx, |this, cx| this.switch_profile(*value, cx)),
      }
    })
    .detach();

    let _vars_event_sub = cx.subscribe_in(&vars, window, move |this, vars, event, window, cx| {
      match event {
        ProjectVariablesEvent::ProfilesChanged => {
          this.update_profiles_select_state(vars.read(cx).profile_infos(), window, cx);
        }
        ProjectVariablesEvent::ActiveProfile(id) => {
          this.update_selected_profile_select_state(id, window, cx);
        }
        _ => {}
      }
    });

    let this = Self {
      focus_handle: cx.focus_handle(),
      _vars_event_sub,
      profile_select_state,
    };

    this.update_profiles_select_state(vars.read(cx).profile_infos(), window, cx);
    this.update_selected_profile_select_state(&vars.read(cx).active_profile(), window, cx);

    this
  }

  fn update_profiles_select_state(&self, profiles: Vec<ProfileInfo>, window: &mut Window, cx: &mut Context<Self>) {
    self
      .profile_select_state
      .update(cx, |state, cx| state.set_items(profiles, window, cx));
  }

  fn update_selected_profile_select_state(&self, id: &Option<Uuid>, window: &mut Window, cx: &mut Context<Self>) {
    match id {
      None => self
        .profile_select_state
        .update(cx, |state, cx| state.set_selected_index(None, window, cx)),
      Some(id) => {
        self
          .profile_select_state
          .update(cx, |state, cx| state.set_selected_value(id, window, cx));
      }
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
      .pb_2()
      .min_w_56()
      .child("Profile")
      .child(Select::new(&self.profile_select_state).small())
  }
}
