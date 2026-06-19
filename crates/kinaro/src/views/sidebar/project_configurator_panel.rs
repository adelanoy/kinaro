use crate::views::sidebar::profile_editor::ProfileVariableEditor;
use crate::workspace::variable::WorkspaceProfile;
use crate::workspace::{ShowProfilesPanel, Workspace, WorkspaceProject, WorkspaceProjectEvent};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable, IconName, IndexPath, Sizable};
use ki_assets::icon::IconAsset;

pub(super) struct ProjectConfigurator {
    focus_handle: FocusHandle,
    active_project: Option<Entity<WorkspaceProject>>,
    profile_select_state: Entity<SelectState<Vec<WorkspaceProfile>>>,
    profile_variable_editor: Entity<ProfileVariableEditor>,
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

        let profile_variable_editor =
            cx.new(|cx| ProfileVariableEditor::new(active_project.clone(), window, cx));

        let mut this = Self {
            focus_handle: cx.focus_handle(),
            active_project,
            profile_select_state,
            profile_variable_editor,
            _profile_change_sub: None,
        };

        this.update_profiles_select_state(window, cx);
        this.setup_profile_change_subscription(window, cx);

        // On project change subscription
        cx.subscribe_in(
            &workspace,
            window,
            |this, workspace, event, window, cx| match event {
                _ => {
                    let active_project = workspace.read(cx).active_project();
                    this.profile_variable_editor
                        .update(cx, |this, cx| this.set_project(active_project.clone(), cx));
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
                let profiles = project.data().variables.profiles.iter()
                    .map(|p| p.clone())
                    .collect::<Vec<WorkspaceProfile>>();
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

    fn on_edit_profiles_variables_action(
        &mut self,
        _: &ShowProfilesPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_project.is_none() {
            return;
        };
        self.profile_variable_editor
            .update(cx, |this, cx| this.show(window, cx));
    }
}

impl Render for ProjectConfigurator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.active_project.is_none();

        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_edit_profiles_variables_action))
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
                            .secondary()
                            .label("Profiles")
                            .icon(IconAsset::Variable)
                            .with_size(gpui_component::Size::Small)
                            .disabled(disabled)
                            .tooltip("Edit the profiles and variables for this project")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.on_edit_profiles_variables_action(
                                    &ShowProfilesPanel,
                                    window,
                                    cx,
                                );
                            })),
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
                            .secondary()
                            .icon(IconName::FolderOpen)
                            .label("Endpoints")
                            .flex_1()
                            .with_size(gpui_component::Size::Small)
                            .disabled(disabled),
                    )
                    .child(
                        Button::new("btn-project-conf")
                            .secondary()
                            .icon(IconName::FolderOpen)
                            .label("Configuration")
                            .flex_1()
                            .with_size(gpui_component::Size::Small)
                            .disabled(disabled),
                    ),
            )
    }
}

impl Focusable for ProjectConfigurator {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
