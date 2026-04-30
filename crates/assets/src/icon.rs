use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use gpui_component::IconNamed;

#[derive(IntoElement, Clone)]
pub enum Icon {
    Plus,
}

impl IconNamed for Icon {
    fn path(self) -> SharedString {
        match self {
            Icon::Plus => "icons/plus.svg",
        }.into()
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        gpui_component::Icon::new(self)
    }
}