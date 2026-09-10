pub mod font;
pub mod icon;
pub mod theme;

use anyhow::anyhow;
use gpui_kit::{App, AssetSource, Result, SharedString};
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "../../assets"]
pub struct Assets;

impl AssetSource for Assets {
  fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
    if path.is_empty() {
      return Ok(None);
    }

    Self::get(path)
      .map(|f| Some(f.data))
      .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
  }

  fn list(&self, path: &str) -> Result<Vec<SharedString>> {
    Ok(Self::iter()
      .filter_map(|p| p.starts_with(path).then(|| p.into()))
      .collect())
  }
}

pub fn init(cx: &mut App) {
  font::init(cx);
  theme::init(cx);
}
