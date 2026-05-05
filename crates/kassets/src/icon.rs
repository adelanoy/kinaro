use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use gpui_component::IconNamed;

#[derive(IntoElement, Clone)]
pub enum IconAsset {
    Plus,
    SidebarCollapsed,
    SidebarOpen,
}

impl IconNamed for IconAsset {
    fn path(self) -> SharedString {
        match self {
            IconAsset::Plus => "icons/plus.svg",
            IconAsset::SidebarCollapsed => "icons/sidebar-collapsed.svg",
            IconAsset::SidebarOpen => "icons/sidebar-open.svg",
        }.into()
    }
}

impl RenderOnce for IconAsset {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        gpui_component::Icon::new(self)
    }
}