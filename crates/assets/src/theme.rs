use std::borrow::Cow;
use gpui::App;
use anyhow::Result;

pub enum ThemeAsset {
    Ayu
}

impl ThemeAsset {
    pub fn load_theme_asset(self, cx: &mut App) -> Result<Option<Cow<'static, [u8]>>> {
        let path = match self {
            ThemeAsset::Ayu => "themes/ayu.json"
        };
        cx.asset_source().load(path)
    }
}