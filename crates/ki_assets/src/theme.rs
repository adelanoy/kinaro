use gpui::App;
use gpui_component::{Theme, ThemeRegistry};
use ki_settings::app_state::AppState;
use log::warn;
use std::borrow::Cow;

#[derive(Debug)]
pub enum ThemeAsset {
    Ayu,
}

impl ThemeAsset {
    fn load_theme_asset(self, cx: &App) -> Cow<'static, [u8]> {
        let path = match self {
            ThemeAsset::Ayu => "themes/ayu.json",
        };
        cx.asset_source()
            .load(path)
            .unwrap_or_else(|e| panic!("Font loaded: {e:?}"))
            .unwrap()
    }
}

pub(crate) fn init(cx: &mut App) {
    let theme_asset = ThemeAsset::Ayu.load_theme_asset(cx);
    if let Err(err) =
        ThemeRegistry::global_mut(cx).load_themes_from_str(str::from_utf8(&theme_asset).unwrap())
    {
        warn!("Error while loading themes: {err}");
        return;
    }

    if let Some(theme) = ThemeRegistry::global(cx).themes().get("Ayu Light").cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }
    if let Some(theme) = ThemeRegistry::global(cx).themes().get("Ayu Dark").cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }
    let theme_prefs = AppState::read(cx, |app_state| app_state.theme);
    Theme::change(theme_prefs, None, cx);
}
