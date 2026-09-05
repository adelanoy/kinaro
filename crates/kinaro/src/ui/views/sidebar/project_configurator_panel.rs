use crate::ui::views::sidebar::project_configuration::ProjectConfigurationTabs;
use crate::workspace::variable::{ProfileInfo, ProjectVariablesEvent};
use crate::workspace::{Project, ProjectEvent};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::{ActiveTheme, IndexPath, Sizable, h_flex, v_flex};
use gpui_kit::*;
use uuid::Uuid;

pub(super) struct ProjectConfigurator {
    focus_handle: FocusHandle,
    _project_event_sub: Subscription,
    _project_vars_event_sub: Subscription,
    profile_select_state: Entity<SelectState<Vec<ProfileInfo>>>,
    project_config_tabs: Entity<ProjectConfigurationTabs>,
}

impl ProjectConfigurator {
    pub(super) fn new(
        project: Entity<Project>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let profile_select_state =
            cx.new(|cx| SelectState::new(vec![], Some(IndexPath::default()), window, cx));
        cx.subscribe(&profile_select_state, {
            let project = project.clone();
            move |_, _, event: &SelectEvent<Vec<ProfileInfo>>, cx| match event {
                SelectEvent::Confirm(value) => {
                    project.update(cx, |this, cx| this.switch_profile(value.to_owned(), cx))
                }
            }
        })
        .detach();

        let _project_event_sub =
            cx.subscribe_in(&project, window, move |this, _, event, window, cx| {
                if let ProjectEvent::ActiveProfileChanged(id) = event {
                    this.update_selected_profile_select_state(id, window, cx)
                }
            });
        let project_vars = project.read(cx).variables.clone();
        let _project_vars_event_sub = cx.subscribe_in(
            &project_vars,
            window,
            move |this, project_vars, event, window, cx| {
                if let ProjectVariablesEvent::ProfilesChanged = event {
                    let profiles = project_vars.read(cx).profile_infos();
                    this.profile_select_state
                        .update(cx, |state, cx| state.set_items(profiles, window, cx));
                }
            },
        );

        let project_config_tabs =
            cx.new(|cx| ProjectConfigurationTabs::new(project.clone(), window, cx));

        let this = Self {
            focus_handle: cx.focus_handle(),
            _project_event_sub,
            _project_vars_event_sub,
            profile_select_state,
            project_config_tabs,
        };

        this.update_profiles_select_state(project_vars.read(cx).profile_infos(), window, cx);

        this
    }

    fn update_profiles_select_state(
        &self,
        profiles: Vec<ProfileInfo>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.profile_select_state
            .update(cx, |state, cx| state.set_items(profiles, window, cx));
    }

    fn update_selected_profile_select_state(
        &self,
        id: &Option<Uuid>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match id {
            None => self
                .profile_select_state
                .update(cx, |state, cx| state.set_selected_index(None, window, cx)),
            Some(id) => {
                self.profile_select_state
                    .update(cx, |state, cx| state.set_selected_value(id, window, cx));
            }
        }
    }
}

impl Render for ProjectConfigurator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .px_1()
            .child(
                h_flex()
                    .text_sm()
                    .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                    .h_8()
                    .child("Project configuration"),
            )
            .child(
                h_flex()
                    .text_sm()
                    .gap_x_2()
                    .pb_2()
                    .child("Profile")
                    .child(Select::new(&self.profile_select_state).small()),
            )
            .child(Separator::horizontal())
            .child(self.project_config_tabs.clone())
    }
}

impl Focusable for ProjectConfigurator {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
