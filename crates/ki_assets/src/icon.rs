use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{App, IntoElement, RenderOnce, SharedString, Window};

#[derive(IntoElement, Clone)]
pub enum IconAsset {
    Ban,
    Profile,
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
            IconAsset::Profile => "icons/profile.svg",
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
        Icon::new(self)
    }
}
