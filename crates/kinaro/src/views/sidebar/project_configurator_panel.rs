use crate::workspace::variable::WorkspaceProfile;
use crate::workspace::{Workspace, WorkspaceProject, WorkspaceProjectEvent};
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
};
use gpui_component::button::Button;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable, IconName, IndexPath, Sizable};

pub(super) struct ProjectConfigurator {
    active_project: Option<Entity<WorkspaceProject>>,
    profile_select_state: Entity<SelectState<Vec<WorkspaceProfile>>>,
    _profile_change_sub: Option<Subscription>,
}

impl ProjectConfigurator {
    pub(super) fn new(
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let active_project = workspace.read(cx).active_project();

        let profile_select_state =
            cx.new(|cx| SelectState::new(vec![], Some(IndexPath::default()), window, cx));
        cx.subscribe(&profile_select_state, {
            let workspace = workspace.clone();
            move |_, _, event: &SelectEvent<Vec<WorkspaceProfile>>, cx| match event {
                SelectEvent::Confirm(value) => workspace.update(cx, |workspace, cx| {
                    workspace.switch_profile(value.to_owned(), cx)
                }),
            }
        })
        .detach();

        let mut this = Self {
            active_project,
            profile_select_state,
            _profile_change_sub: None,
        };

        this.update_profiles_select_state(window, cx);
        this.setup_profile_change_subscription(window, cx);

        cx.subscribe_in(
            &workspace,
            window,
            |this, workspace, event, window, cx| match event {
                _ => {
                    let active_project = workspace.read(cx).active_project();
                    this.active_project = active_project;
                    this.update_profiles_select_state(window, cx);
                    this.setup_profile_change_subscription(window, cx);
                }
            },
        )
        .detach();

        this
    }

    fn setup_profile_change_subscription(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._profile_change_sub = match &self.active_project {
            None => None,
            Some(active_project) => Some(cx.subscribe_in(
                &active_project,
                window,
                move |this, _, event, window, cx| match event {
                    WorkspaceProjectEvent::ProfilesModified => {
                        this.update_profiles_select_state(window, cx);
                    }
                },
            )),
        }
    }

    fn update_profiles_select_state(&self, window: &mut Window, cx: &mut Context<Self>) {
        let active_project = &self.active_project;
        let (profiles, profile_index) = match active_project {
            Some(active_project) => active_project.read_with(cx, |project, _| {
                let active_profile_id = project.active_profile();
                let profiles = project.data().variables.profiles.clone();
                let selected_profile_index = profiles
                    .iter()
                    .position(|p| Some(p.id) == active_profile_id)
                    .unwrap_or(0);
                (profiles, selected_profile_index)
            }),
            None => (vec![], 0),
        };

        self.profile_select_state.update(cx, |state, cx| {
            state.set_items(profiles, window, cx);
            state.set_selected_index(Some(IndexPath::new(profile_index)), window, cx);
        });
    }
}

impl Render for ProjectConfigurator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.active_project.is_none();

        v_flex()
            .w_full()
            .child(
                h_flex()
                    .text_xs()
                    .text_color(cx.theme().sidebar_foreground.opacity(0.7))
                    .h_8()
                    .child("Project configuration"),
            )
            .child(
                h_flex()
                    .text_xs()
                    .gap_x_2()
                    .pb_2()
                    .child(
                        Button::new("btn-profiles-var")
                            .icon(IconName::FolderOpen)
                            .outline()
                            .label("Profiles")
                            .with_size(gpui_component::Size::Small)
                            .disabled(disabled),
                    )
                    .child(
                        Select::new(&self.profile_select_state)
                            .with_size(gpui_component::Size::Small)
                            .disabled(disabled),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_x_2()
                    .child(
                        Button::new("btn-endpoint-conf")
                            .icon(IconName::FolderOpen)
                            .label("Endpoints")
                            .flex_1()
                            .with_size(gpui_component::Size::Small)
                            .disabled(disabled),
                    )
                    .child(
                        Button::new("btn-project-conf")
                            .icon(IconName::FolderOpen)
                            .label("Configuration")
                            .flex_1()
                            .with_size(gpui_component::Size::Small)
                            .disabled(disabled),
                    ),
            )
    }
}
