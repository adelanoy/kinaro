use crate::views::{
    AppendProject, CreateProject, DeleteProject, RenameProject, SwitchActiveProject,
};
use crate::workspace::Workspace;
use gpui::{
    div, px, AppContext, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window,
};
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

    fn on_rename_project(
        this: &mut ProjectSelector,
        action: &RenameProject,
        window: &mut Window,
        cx: &mut Context<ProjectSelector>,
    ) {
        let workspace = this.workspace.clone();
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
                            DialogClose::new()
                                .child(Button::new("cancel").label("Cancel").outline()),
                        )
                        .child(
                            DialogAction::new()
                                .child(Button::new("confirm").primary().label("Rename")),
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
                                window.push_notification(
                                    Notification::success("Project renamed"),
                                    cx,
                                );
                            }
                        });
                        true
                    }
                })
        })
    }
}

impl Render for ProjectSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_project_name = self.workspace.read_with(cx, |w, _| {
            w.get_active_project()
                .map_or(SharedString::new("--"), |p| p.name.clone())
        });

        div().on_action(cx.listener(Self::on_rename_project)).child(
            Button::new("btn-project-selector")
                .primary()
                .min_w(px(150.))
                .dropdown_caret(true)
                .label(current_project_name)
                .dropdown_menu({
                    let (current_project_id, projects) = self.workspace.read_with(cx, |w, _| {
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
                            submenu = submenu.submenu_with_icon(submenu_icon, name, window, cx, {
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
                                        Box::new(DeleteProject(project_id)),
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
                                Box::new(AppendProject),
                            )
                    }
                }),
        )
    }
}
