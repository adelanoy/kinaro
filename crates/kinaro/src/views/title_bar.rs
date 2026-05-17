use gpui::*;
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, StyledExt, Theme, ThemeMode, TitleBar};
use settings::app_state::AppState;

pub(crate) struct AppTitleBar {}

impl AppTitleBar {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark_mode = cx.theme().mode == ThemeMode::Dark;

        TitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(Label::new("Kinaro").font_semibold().text_2xl()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(Icon::new(IconName::Moon))
                    .child(
                        Switch::new("theme-mode-selector")
                            .checked(is_dark_mode)
                            .small()
                            .on_click(|checked, _, cx| {
                                let mode = if *checked {
                                    ThemeMode::Dark
                                } else {
                                    ThemeMode::Light
                                };
                                Theme::change(mode, None, cx);
                                cx.refresh_windows();
                                AppState::update(cx, |app_state, _cx| {
                                    app_state.theme = mode;
                                    true
                                })
                            }),
                    ),
            )
    }
}
