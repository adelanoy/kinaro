use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use gpui_component::IconNamed;

#[derive(IntoElement, Clone)]
pub enum IconAsset {
    ChevronUpDown,
    Plus,
    Refresh,
    Rename,
    SidebarCollapsed,
    SidebarOpen,
    Switch,
    Tree,
    Variable,
}

impl IconNamed for IconAsset {
    fn path(self) -> SharedString {
        match self {
            IconAsset::ChevronUpDown => "icons/chevron-up-down.svg",
            IconAsset::Plus => "icons/plus.svg",
            IconAsset::Refresh => "icons/refresh.svg",
            IconAsset::Rename => "icons/rename.svg",
            IconAsset::SidebarCollapsed => "icons/sidebar-collapsed.svg",
            IconAsset::SidebarOpen => "icons/sidebar-open.svg",
            IconAsset::Switch => "icons/switch.svg",
            IconAsset::Tree => "icons/tree.svg",
            IconAsset::Variable => "icons/variable.svg",
        }
        .into()
    }
}

impl RenderOnce for IconAsset {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        gpui_component::Icon::new(self)
    }
}
