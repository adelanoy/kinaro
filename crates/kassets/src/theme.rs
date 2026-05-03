use gpui::App;
use std::borrow::Cow;
use gpui_component::{Theme, ThemeRegistry};
use log::warn;
use settings::app_state::AppState;

#[derive(Debug)]
pub enum ThemeAsset {
    Ayu
}

impl ThemeAsset {
    fn load_theme_asset(self, cx: &App) -> Cow<'static, [u8]> {
        let path = match self {
            ThemeAsset::Ayu => "themes/ayu.json"
        };
        cx.asset_source().load(path).expect(&format!("Font loaded: {:?}", self)).unwrap()
    }
}

pub(crate) fn init(cx: &mut App) {
    let theme_asset = ThemeAsset::Ayu.load_theme_asset(cx);
    if let Err(err) =
        ThemeRegistry::global_mut(cx).load_themes_from_str(str::from_utf8(&*theme_asset).unwrap())
    {
        warn!("Error while loading themes: {}", err);
        return;
    }

    let theme_prefs = AppState::read(cx, |app_state| app_state.theme.clone());
    if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_prefs).cloned() {
        Theme::global_mut(cx).apply_config(&theme);
    }
}