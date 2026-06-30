use crate::views::sidebar::project_configuration::ProjectConfigurationTabs;
use crate::workspace::variable::WorkspaceProfile;
use crate::workspace::{WorkspaceProject, WorkspaceProjectEvent};
use gpui::*;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::separator::Separator;
use gpui_component::{ActiveTheme, IndexPath, Sizable, h_flex, v_flex};
use uuid::Uuid;

pub(super) struct ProjectConfigurator {
    focus_handle: FocusHandle,
    project: Entity<WorkspaceProject>,
    _project_event_sub: Subscription,
    profile_select_state: Entity<SelectState<Vec<WorkspaceProfile>>>,
    _profile_change_sub: Option<Subscription>,
    project_config_tabs: Entity<ProjectConfigurationTabs>,
}

impl ProjectConfigurator {
    pub(super) fn new(
        project: Entity<WorkspaceProject>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let profile_select_state =
            cx.new(|cx| SelectState::new(vec![], Some(IndexPath::default()), window, cx));
        cx.subscribe(&profile_select_state, {
            let project = project.clone();
            move |_, _, event: &SelectEvent<Vec<WorkspaceProfile>>, cx| match event {
                SelectEvent::Confirm(value) => {
                    project.update(cx, |this, cx| this.switch_profile(value.to_owned(), cx))
                }
            }
        })
        .detach();
        let _project_event_sub = cx.subscribe_in(
            &project,
            window,
            move |this, _, event, window, cx| match event {
                WorkspaceProjectEvent::ActiveProfileChanged(id) => {
                    this.update_selected_profile_select_state(id, window, cx)
                }
                WorkspaceProjectEvent::ProfilesChanged => {
                    this.update_profiles_select_state(window, cx)
                }
            },
        );

        let project_config_tabs =
            cx.new(|cx| ProjectConfigurationTabs::new(project.clone(), window, cx));

        let this = Self {
            focus_handle: cx.focus_handle(),
            project,
            _project_event_sub,
            profile_select_state,
            _profile_change_sub: None,
            project_config_tabs,
        };

        this.update_profiles_select_state(window, cx);

        this
    }

    fn update_profiles_select_state(&self, window: &mut Window, cx: &mut Context<Self>) {
        let active_project = &self.project;
        let (profiles, profile_index) = active_project.read_with(cx, |project, _| {
            let active_profile_id = project.active_profile();
            let profiles = project
                .data()
                .variables
                .profiles
                .iter()
                .map(|p| p.clone())
                .collect::<Vec<WorkspaceProfile>>();
            let selected_profile_index = profiles
                .iter()
                .position(|p| Some(p.id) == active_profile_id)
                .unwrap_or(0);
            (profiles, selected_profile_index)
        });

        self.profile_select_state.update(cx, |state, cx| {
            state.set_items(profiles, window, cx);
            state.set_selected_index(Some(IndexPath::new(profile_index)), window, cx);
        });
    }

    fn update_selected_profile_select_state(
        &self,
        id: &Option<Uuid>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match id {
            None => self.profile_select_state.update(cx, |state, cx| {
                state.set_selected_index(None, window, cx);
            }),
            Some(id) => {
                let pos = self.project.read_with(cx, |project, _| {
                    project
                        .data()
                        .variables
                        .profiles
                        .iter()
                        .position(|p| p.id == *id)
                });
                self.profile_select_state.update(cx, |state, cx| match pos {
                    None => state.set_selected_index(None, window, cx),
                    Some(pos) => state.set_selected_index(Some(IndexPath::new(pos)), window, cx),
                });
            }
        }
    }
}

impl Render for ProjectConfigurator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .child(
                h_flex()
                    .text_xs()
                    .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                    .h_8()
                    .child("Project configuration"),
            )
            .child(h_flex().text_xs().gap_x_2().pb_2().child("Profile").child(
                Select::new(&self.profile_select_state).with_size(gpui_component::Size::Small),
            ))
            .child(Separator::horizontal())
            .child(self.project_config_tabs.clone())
    }
}

impl Focusable for ProjectConfigurator {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
