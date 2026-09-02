use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use gpui_component::IconNamed;

#[derive(IntoElement, Clone)]
pub enum IconAsset {
    Ban,
    Refresh,
    Rename,
    SidebarCollapsed,
    SidebarOpen,
    TestCase,
    TestStep,
    TestSuite,
    Tree,
    Variable,
}

impl IconNamed for IconAsset {
    fn path(self) -> SharedString {
        match self {
            IconAsset::Ban => "icons/ban.svg",
            IconAsset::Refresh => "icons/refresh.svg",
            IconAsset::Rename => "icons/rename.svg",
            IconAsset::SidebarCollapsed => "icons/sidebar-collapsed.svg",
            IconAsset::SidebarOpen => "icons/sidebar-open.svg",
            IconAsset::TestCase => "icons/test-case.svg",
            IconAsset::TestStep => "icons/test-step.svg",
            IconAsset::TestSuite => "icons/test-suite.svg",
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
