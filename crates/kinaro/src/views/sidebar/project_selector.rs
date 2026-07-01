use crate::workspace::project::PROJECT_FILE_EXT;
use crate::workspace::{
    CreateProject, OpenProject, RemoveProject, RenameProject, SwitchActiveProject, Workspace,
    WorkspaceProjectInfo,
};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_component::form::{field, v_form};
use gpui_component::input::{Input, InputState};
use gpui_component::menu::DropdownMenu;
use gpui_component::notification::Notification;
use gpui_component::{ActiveTheme, Disableable, IconName, Sizable, WindowExt, h_flex, v_flex};
use ki_assets::icon::IconAsset;
use ki_settings::app_state::AppState;
use std::path::PathBuf;

pub(super) struct ProjectSelector {
    workspace: Entity<Workspace>,
}

impl ProjectSelector {
    pub(super) fn new(workspace: Entity<Workspace>) -> Self {
        Self { workspace }
    }
}

impl ProjectSelector {
    fn on_switch_project(
        &mut self,
        action: &SwitchActiveProject,
        window: &mut Window,
        cx: &mut Context<ProjectSelector>,
    ) {
        let project_path = action.0.clone();
        if let Err(err) = self
            .workspace
            .update(cx, move |this, cx| this.switch_project(project_path, cx))
        {
            window.push_notification(err, cx);
        }
    }

    fn on_open_project(
        &mut self,
        _: &OpenProject,
        window: &mut Window,
        cx: &mut Context<ProjectSelector>,
    ) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(result) = path.await else {
                return;
            };
            if let Some(paths) = result.ok().flatten() {
                if !paths.is_empty() {
                    let window_handle = cx.window_handle();
                    _ = this.update(cx, |this, cx| {
                        if let Err(err) = this
                            .workspace
                            .update(cx, |this, cx| this.open_project(paths[0].clone(), cx))
                        {
                            _ = window_handle.update(cx, |_, window, cx| {
                                window.push_notification(format!("{:?}", err), cx);
                            });
                        }
                        AppState::update(cx, |state, _| {
                            state.last_dir_path = paths[0].clone();
                            true
                        });
                    });
                }
            }
        })
        .detach();
    }
    fn on_rename_project(
        &mut self,
        action: &RenameProject,
        window: &mut Window,
        cx: &mut Context<ProjectSelector>,
    ) {
        let workspace = self.workspace.clone();
        let project_path = action.0.clone();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(action.1.clone(), window, cx);
            state
        });

        window.open_dialog(cx, move |dialog, _, cx| {
            dialog
                .title("Change Project Name")
                .child(
                    v_flex()
                        .gap_3()
                        .child("Enter the project's name:")
                        .child(Input::new(&input)),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel").label("Cancel").outline()),
                        )
                        .child(
                            DialogAction::new()
                                .child(Button::new("confirm").primary().label("Rename")),
                        ),
                )
                .on_ok({
                    let name = input.clone().read(cx).value();
                    let project_path = project_path.clone();
                    let workspace = workspace.clone();
                    move |_, _, cx| {
                        workspace.update(cx, |workspace, cx| {
                            workspace.rename_project(&project_path, name.clone(), cx)
                        });
                        true
                    }
                })
        });
    }

    fn on_remove_project(
        &mut self,
        action: &RemoveProject,
        window: &mut Window,
        cx: &mut Context<ProjectSelector>,
    ) {
        let project_path = &action.0;
        self.workspace.update(cx, |workspace, cx| {
            workspace.remove_project(project_path, cx);
            window.push_notification(Notification::success("Project has been removed"), cx);
        });
    }

    fn on_create_project(
        &mut self,
        _: &CreateProject,
        window: &mut Window,
        cx: &mut Context<ProjectSelector>,
    ) {
        let workspace = self.workspace.clone();
        let name_input = cx.new(|cx| InputState::new(window, cx));
        let path_input = cx.new(|cx| InputState::new(window, cx));
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog
                .title("Create Project")
                .child(
                    v_form()
                        .layout(Axis::Horizontal)
                        .label_width(px(100.))
                        .with_size(gpui_component::Size::Small)
                        .child(
                            field()
                                .label("Name")
                                .child(Input::new(&name_input))
                                .required(true),
                        )
                        .child(
                            field().label("Path").required(true).child(
                                h_flex()
                                    .gap_2()
                                    .border_1()
                                    .border_color(cx.theme().input)
                                    .bg(cx.theme().input_background())
                                    .rounded(cx.theme().radius)
                                    .child(
                                        div().flex_1().child(
                                            Input::new(&path_input).pl_0().appearance(false),
                                        ),
                                    )
                                    .child(
                                        Button::new("file")
                                            .ghost()
                                            .icon(IconName::FolderOpen)
                                            .on_click(prompt_to_save_project(
                                                path_input.clone(),
                                                cx,
                                            )),
                                    ),
                            ),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel").label("Cancel").outline()),
                        )
                        .child(DialogAction::new().child(
                            Button::new("confirm").primary().label("Create").disabled(
                                name_input.read(cx).value().len() == 0
                                    || path_input.read_with(cx, |state, _| {
                                        state.value().len() == 0
                                            || !state.value().ends_with(PROJECT_FILE_EXT)
                                    }),
                            ),
                        )),
                )
                .on_ok({
                    let name = name_input.clone();
                    let path = path_input.clone();
                    let workspace = workspace.clone();
                    move |_, window, cx| {
                        let project_name = name.read(cx).value();
                        let project = PathBuf::from(path.read(cx).value().to_string());
                        let project_dir = project.parent();
                        if project_dir.is_none() || !project_dir.unwrap().exists() {
                            window.push_notification(
                                Notification::error("Cannot create project: invalid location"),
                                cx,
                            );
                            return false;
                        }

                        workspace.update(cx, |workspace, cx| {
                            workspace.create_project(project_name, project, window, cx)
                        });
                        true
                    }
                })
        });
    }
}

impl Render for ProjectSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_project = self.workspace.read(cx).active_project();
        let current_project_name = if let Some(project) = &active_project {
            project.read(cx).name.clone()
        } else {
            SharedString::new("--")
        };

        div()
            .p_2()
            .on_action(cx.listener(Self::on_rename_project))
            .on_action(cx.listener(Self::on_open_project))
            .on_action(cx.listener(Self::on_switch_project))
            .on_action(cx.listener(Self::on_remove_project))
            .on_action(cx.listener(Self::on_create_project))
            .w_full()
            .child(
                Button::new("btn-project-selector")
                    .primary()
                    .h_12()
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .size_full()
                            .items_center()
                            .justify_between()
                            .child(current_project_name)
                            .child(IconName::ChevronsUpDown),
                    )
                    .dropdown_menu({
                        let project_infos = self.workspace.read(cx).all_project_infos();

                        move |this, window, cx| {
                            let project_infos = project_infos.clone();
                            let mut submenu = this;
                            for project_info in project_infos {
                                let WorkspaceProjectInfo {
                                    name,
                                    path,
                                    loaded,
                                    active,
                                } = project_info;
                                let submenu_icon = if active {
                                    Some(IconName::Check.into())
                                } else {
                                    None
                                };
                                submenu = submenu.submenu_with_icon(
                                    submenu_icon,
                                    name.clone(),
                                    window,
                                    cx,
                                    {
                                        let name = name.clone();
                                        move |this, _, _| {
                                            this.menu_with_icon_and_disabled(
                                                "Switch To",
                                                IconAsset::Switch,
                                                Box::new(SwitchActiveProject(path.clone())),
                                                active,
                                            )
                                            .separator()
                                            .menu_with_icon(
                                                "Delete",
                                                IconName::Delete,
                                                Box::new(RemoveProject(path.clone())),
                                            )
                                            .menu_with_icon(
                                                "Rename",
                                                IconAsset::Rename,
                                                Box::new(RenameProject(path.clone(), name.clone())),
                                            )
                                        }
                                    },
                                );
                            }
                            submenu
                                .separator()
                                .menu_with_icon(
                                    "Create new project",
                                    IconName::Plus,
                                    Box::new(CreateProject),
                                )
                                .menu_with_icon(
                                    "Open project",
                                    IconName::FolderOpen,
                                    Box::new(OpenProject),
                                )
                        }
                    }),
            )
    }
}

fn prompt_to_save_project(
    path_input: Entity<InputState>,
    cx: &mut App,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let last_path = AppState::read(cx, |state| state.last_dir_path.clone());
    move |_, window, cx| {
        let path =
            cx.prompt_for_new_path(&last_path, Some(&format!("project.{}", PROJECT_FILE_EXT)));
        window
            .spawn(cx, {
                let path_input = path_input.clone();
                async move |cx| {
                    let Ok(result) = path.await else {
                        return;
                    };
                    if let Some(path) = result.ok().flatten() {
                        _ = cx.window_handle().update(cx, |_, window, cx| {
                            let path_str = SharedString::new(path.to_string_lossy());
                            path_input
                                .update(cx, |state, cx| state.set_value(path_str, window, cx));
                            AppState::update(cx, |state, _| {
                                state.last_dir_path = path.parent().unwrap().to_owned();
                                true
                            });
                        });
                    }
                }
            })
            .detach();
    }
}