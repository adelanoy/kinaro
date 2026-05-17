use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use gpui_component::IconNamed;

#[derive(IntoElement, Clone)]
pub enum IconAsset {
    ChevronUpDown,
    Delete,
    Plus,
    Rename,
    SidebarCollapsed,
    SidebarOpen,
    Switch,
}

impl IconNamed for IconAsset {
    fn path(self) -> SharedString {
        match self {
            IconAsset::ChevronUpDown => "icons/chevron-up-down.svg",
            IconAsset::Delete => "icons/delete.svg",
            IconAsset::Plus => "icons/plus.svg",
            IconAsset::Rename => "icons/rename.svg",
            IconAsset::SidebarCollapsed => "icons/sidebar-collapsed.svg",
            IconAsset::SidebarOpen => "icons/sidebar-open.svg",
            IconAsset::Switch => "icons/switch.svg",
        }
        .into()
    }
}

impl RenderOnce for IconAsset {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        gpui_component::Icon::new(self)
    }
}
