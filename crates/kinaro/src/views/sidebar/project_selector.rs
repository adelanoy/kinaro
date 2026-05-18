use crate::workspace::{
    CreateProject, RemoveProject, OpenProject, RenameProject, SwitchActiveProject, Workspace,
};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::{DialogAction, DialogClose, DialogFooter};
use gpui_component::input::{Input, InputState};
use gpui_component::menu::DropdownMenu;
use gpui_component::notification::Notification;
use gpui_component::{v_flex, IconName, WindowExt};
use kassets::icon::IconAsset;

pub(super) struct ProjectSelector {
    workspace: Entity<Workspace>,
}

impl ProjectSelector {
    pub(super) fn new(_cx: &mut Context<Self>, workspace: Entity<Workspace>) -> Self {
        Self { workspace }
    }
}

impl Render for ProjectSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_project_name = self.workspace.read_with(cx, |w, _| {
            w.get_active_project()
                .map_or(SharedString::new("--"), |p| p.name.clone())
        });

        div()
            .on_action(cx.listener(on_rename_project))
            .on_action(cx.listener(on_open_project))
            .on_action(cx.listener(on_switch_project))
            .on_action(cx.listener(on_remove_project))
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
                            self.workspace.read_with(cx, |w, _| {
                                let current_project_id = w.get_active_project_id();
                                let project_infos = w
                                    .projects
                                    .iter()
                                    .map(|p| (p.id, p.name.clone()))
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
                    cx.notify();
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
                    .child("Enter the new project's name:")
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
                move |_, window, cx| {
                    _ = workspace.update(cx, |workspace, cx| {
                        let name = input.read_with(cx, |input, _| input.value());
                        if let Some(current_project) =
                            workspace.projects.iter_mut().find(|p| p.id == project_id)
                        {
                            current_project.name = name;
                            window.push_notification(Notification::success("Project renamed"), cx);
                        }
                    });
                    true
                }
            })
    })
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