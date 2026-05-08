use crate::actions::DeleteProject;
use gpui::{Context, Entity, IntoElement, Render, Styled, Window, px};
use gpui_component::button::Button;
use gpui_component::menu::DropdownMenu;
use kassets::icon::IconAsset;
use kworkspace::Workspace;
use std::rc::Rc;
use uuid::Uuid;

pub(super) struct ProjectSelector {
    workspace: Rc<Entity<Workspace>>,
}

impl ProjectSelector {
    pub(super) fn new(_cx: &mut Context<Self>, workspace: Rc<Entity<Workspace>>) -> Self {
        Self { workspace }
    }
}

impl Render for ProjectSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (current_project_name, projects) = self.workspace.read_with(cx, |w, _| {
            let current_project_id = w.get_active_project_id();
            let mut current_project_name = None;
            let project_infos = w
                .projects
                .iter()
                .map(|p| {
                    if current_project_id.is_some() && current_project_id == Some(p.id) {
                        current_project_name = Some(p.name.clone());
                    }
                    (p.id, p.name.clone())
                })
                .collect::<Vec<_>>();
            (
                current_project_name.unwrap_or_else(|| "--".to_string()),
                project_infos,
            )
        });

        Button::new("btn")
            .outline()
            .min_w(px(150.))
            .dropdown_caret(true)
            .label(current_project_name)
            .dropdown_menu(move |this, window, cx| {
                let mut menu = this;
                for (id, name) in projects.iter() {
                    menu = menu.submenu(name.clone(), window, cx, |this, _, _| {
                        this.menu_with_icon(
                            "Remove",
                            IconAsset::Delete,
                            Box::new(DeleteProject(*id)),
                        )
                    });
                }
                menu
            })
    }
}
