use gpui::App;
use std::borrow::Cow;

#[derive(Debug)]
pub enum FontAsset {
    Roboto,
    JetbrainsMono,
}

impl FontAsset {
    fn load_font_assets(cx: &App) -> Vec<Cow<'static, [u8]>> {
        [FontAsset::Roboto, FontAsset::JetbrainsMono]
            .into_iter()
            .map(|asset| asset.load_font_asset(cx))
            .collect()
    }

    fn load_font_asset(self, cx: &App) -> Cow<'static, [u8]> {
        let path = match self {
            FontAsset::Roboto => "fonts/Roboto-Regular.ttf",
            FontAsset::JetbrainsMono => "fonts/JetBrainsMonoNL-Regular.ttf",
        };
        cx.asset_source().load(path).expect(&format!("Font loaded: {:?}", self)).unwrap()
    }
}

pub(crate) fn init(cx: &mut App) {
    let fonts = FontAsset::load_font_assets(cx);
    _ = cx.text_system().add_fonts(fonts);
}