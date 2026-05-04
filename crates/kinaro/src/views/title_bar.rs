use gpui::*;
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, StyledExt, Theme, ThemeMode, TitleBar};
use settings::GlobalSettings;

pub(crate) struct AppTitleBar {}

impl AppTitleBar {
    pub(crate) fn new(_cx: &mut Context<Self>) -> Self {
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
                    .child(Icon::new(IconName::Moon).small())
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
                                cx.update_global(|settings: &mut GlobalSettings, cx| {
                                    settings.update_app_state(cx, |app_state, _cx| {
                                        app_state.theme = mode
                                    })
                                })
                            }),
                    ),
            )
    }
}
