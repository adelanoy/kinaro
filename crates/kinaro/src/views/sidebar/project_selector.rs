use crate::workspace::{
    CreateProject, OpenProject, RemoveProject, RenameProject, SwitchActiveProject,
    Workspace, Project, PROJECT_FILE_EXT,
};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_component::form::{field, v_form};
use gpui_component::input::{Input, InputState};
use gpui_component::menu::DropdownMenu;
use gpui_component::notification::Notification;
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable, IconName, Sizable, WindowExt};
use ki_assets::icon::IconAsset;
use ki_settings::app_state::AppState;
use std::path::PathBuf;

pub(super) struct ProjectSelector {
    workspace: Entity<Workspace>,
    active_project: Option<Entity<Project>>,
}

impl ProjectSelector {
    pub(super) fn new(workspace: Entity<Workspace>, cx: &mut Context<Self>) -> Self {
        let active_project = workspace.read(cx).active_project();
        cx.subscribe(&workspace, |this, workspace, event, cx| match event {
            _ => {
                let active_project = workspace.read(cx).active_project();
                this.active_project = active_project.clone();
            }
        })
        .detach();
        Self {
            workspace,
            active_project,
        }
    }
}

impl Render for ProjectSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_project_name = if let Some(project) = &self.active_project {
            project.read(cx).name.clone()
        } else {
            SharedString::new("--")
        };

        div()
            .p_2()
            .on_action(cx.listener(on_rename_project))
            .on_action(cx.listener(on_open_project))
            .on_action(cx.listener(on_switch_project))
            .on_action(cx.listener(on_remove_project))
            .on_action(cx.listener(on_create_project))
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
                        let (current_project_id, projects) =
                            self.workspace.read_with(cx, |w, cx| {
                                let current_project_id = w.active_project_id();
                                let project_infos = w
                                    .projects
                                    .iter()
                                    .map(|(_, p)| p.read_with(cx, |p, _| (p.id, p.name.clone())))
                                    .collect::<Vec<_>>();
                                (current_project_id, project_infos)
                            });

                        move |this, window, cx| {
                            let mut submenu = this;
                            for (id, name) in &projects {
                                let is_active_project = Some(*id) == current_project_id;
                                let submenu_icon = if is_active_project {
                                    Some(IconName::Check.into())
                                } else {
                                    None
                                };
                                submenu =
                                    submenu.submenu_with_icon(submenu_icon, name, window, cx, {
                                        let project_id = *id;
                                        let name = name.clone();
                                        move |this, _, _| {
                                            this.menu_with_icon_and_disabled(
                                                "Switch To",
                                                IconAsset::Switch,
                                                Box::new(SwitchActiveProject(project_id)),
                                                is_active_project,
                                            )
                                            .separator()
                                            .menu_with_icon(
                                                "Delete",
                                                IconName::Delete,
                                                Box::new(RemoveProject(project_id)),
                                            )
                                            .menu_with_icon(
                                                "Rename",
                                                IconAsset::Rename,
                                                Box::new(RenameProject(project_id, name.clone())),
                                            )
                                        }
                                    });
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

fn on_switch_project(
    project_selector: &mut ProjectSelector,
    action: &SwitchActiveProject,
    window: &mut Window,
    cx: &mut Context<ProjectSelector>,
) {
    let project_id = action.0;
    if let Err(err) = project_selector
        .workspace
        .update(cx, move |this, cx| this.switch_project(project_id, cx))
    {
        window.push_notification(Notification::error(format!("{}", err)), cx);
    }
}

fn on_open_project(
    _: &mut ProjectSelector,
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
    project_selector: &mut ProjectSelector,
    action: &RenameProject,
    window: &mut Window,
    cx: &mut Context<ProjectSelector>,
) {
    let workspace = project_selector.workspace.clone();
    let project_id = action.0;
    let input = cx.new(|cx| {
        let mut state = InputState::new(window, cx);
        state.set_value(action.1.clone(), window, cx);
        state
    });

    window.open_dialog(cx, move |dialog, _, _| {
        let workspace = workspace.clone();
        let input = input.clone();
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
                        DialogClose::new().child(Button::new("cancel").label("Cancel").outline()),
                    )
                    .child(
                        DialogAction::new().child(Button::new("confirm").primary().label("Rename")),
                    ),
            )
            .on_ok({
                let input = input.clone();
                move |_, _, cx| {
                    workspace.update(cx, |workspace, cx| {
                        if let Some(project) = workspace.projects.get(&project_id) {
                            let name = input.read_with(cx, |input, _| input.value());
                            project.update(cx, |this, _| this.name = name);
                        }
                    });
                    true
                }
            })
    });
}

fn on_remove_project(
    project_selector: &mut ProjectSelector,
    action: &RemoveProject,
    window: &mut Window,
    cx: &mut Context<ProjectSelector>,
) {
    let project_id = action.0;
    project_selector.workspace.update(cx, |workspace, cx| {
        workspace.remove_project(project_id, cx);
        window.push_notification(Notification::success("Project has been removed"), cx);
    });
}

fn on_create_project(
    project_selector: &mut ProjectSelector,
    _: &CreateProject,
    window: &mut Window,
    cx: &mut Context<ProjectSelector>,
) {
    let workspace = project_selector.workspace.clone();
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
                                    div()
                                        .flex_1()
                                        .child(Input::new(&path_input).pl_0().appearance(false)),
                                )
                                .child(
                                    Button::new("file")
                                        .ghost()
                                        .icon(IconName::FolderOpen)
                                        .on_click(prompt_to_save_ki_project(
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
                        DialogClose::new().child(Button::new("cancel").label("Cancel").outline()),
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
                    let project_name = name.read(cx).value().to_string();
                    let ki_project = PathBuf::from(path.read(cx).value().to_string());
                    let project_dir = ki_project.parent();
                    if project_dir.is_none() || !project_dir.unwrap().exists() {
                        window.push_notification(
                            Notification::error("Cannot create project: invalid location"),
                            cx,
                        );
                        return false;
                    }

                    if let Err(err) = workspace.update(cx, |workspace, cx| {
                        workspace.create_project(project_name, ki_project, cx)
                    }) {
                        window.push_notification(
                            Notification::error(format!("Error saving file: {}", err)),
                            cx,
                        );
                    }
                    true
                }
            })
    });
}

fn prompt_to_save_ki_project(
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
