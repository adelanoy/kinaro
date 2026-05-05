use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, relative};

pub(crate) struct ProjectSidebar {}

impl ProjectSidebar {
    pub(crate) fn new(_cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for ProjectSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(relative(1.0)).child("Sidebar")
    }
}
