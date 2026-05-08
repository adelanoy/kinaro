mod project_selector;

use crate::views::sidebar::project_selector::ProjectSelector;
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
};
use gpui_component::menu::DropdownMenu;
use gpui_component::sidebar::{
    Sidebar, SidebarFooter, SidebarGroup, SidebarMenu, SidebarMenuItem,
};
use gpui_component::Side;
use kworkspace::Workspace;
use std::rc::Rc;

pub(crate) struct ProjectSidebar {
    project_selector: Entity<ProjectSelector>,
}

impl ProjectSidebar {
    pub(crate) fn new(cx: &mut Context<Self>, workspace: Rc<Entity<Workspace>>) -> Self {
        let project_selector = cx.new(|cx| ProjectSelector::new(cx, workspace));
        Self { project_selector }
    }
}

impl Render for ProjectSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Sidebar::new("project-sidebar")
            .side(Side::Left)
            .w_full()
            .header(self.project_selector.clone())
            .child(
                SidebarGroup::new("Navigation").child(
                    SidebarMenu::new()
                        .child(
                            SidebarMenuItem::new("Dashboard")
                                .on_click(|_, _, _| println!("Dashboard clicked")),
                        )
                        .child(
                            SidebarMenuItem::new("Settings")
                                .on_click(|_, _, _| println!("Settings clicked")),
                        ),
                ),
            )
            .footer(SidebarFooter::new().child("User Profile"))
    }
}
