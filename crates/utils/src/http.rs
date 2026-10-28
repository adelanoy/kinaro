use std::sync::LazyLock;
use gpui_kit::{App, Hsla, IntoElement, Rgba, SharedString, Styled, Window};
use gpui_kit::component::label::Label;
use gpui_kit::component::select::SelectItem;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub static HTTP_HEADER_CONTENT_TYPE: LazyLock<SharedString> = LazyLock::new(|| SharedString::new("Content-Type"));
const REST_GET_COLOR: &str = "#38bdf8";
const REST_POST_COLOR: &str = "#fcd34d";
const REST_PUT_COLOR: &str = "#b45309";
const REST_DELETE_COLOR: &str = "#ef4444";
const REST_PATCH_COLOR: &str = "#9333ea";
const REST_OPTIONS_COLOR: &str = "#4ade80";
const REST_HEAD_COLOR: &str = "#ec4899";

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
  Get,
  Post,
  Put,
  Delete,
  Patch,
  Head,
  Options,
}
impl HttpMethod {
  pub const ALL: &'static [HttpMethod] = &[
    HttpMethod::Get,
    HttpMethod::Post,
    HttpMethod::Put,
    HttpMethod::Delete,
    HttpMethod::Patch,
    HttpMethod::Head,
    HttpMethod::Options,
  ];

  pub fn color(&self) -> impl Into<Hsla> {
    let hex_str = match self {
      HttpMethod::Get => REST_GET_COLOR,
      HttpMethod::Post => REST_POST_COLOR,
      HttpMethod::Put => REST_PUT_COLOR,
      HttpMethod::Delete => REST_DELETE_COLOR,
      HttpMethod::Patch => REST_PATCH_COLOR,
      HttpMethod::Head => REST_HEAD_COLOR,
      HttpMethod::Options => REST_OPTIONS_COLOR,
    };
    Rgba::try_from(hex_str).unwrap()
  }
}

impl SelectItem for HttpMethod {
  type Value = HttpMethod;

  fn title(&self) -> SharedString {
    match self {
      HttpMethod::Get => "Get".into(),
      HttpMethod::Post => "Post".into(),
      HttpMethod::Put => "Put".into(),
      HttpMethod::Delete => "Delete".into(),
      HttpMethod::Patch => "Patch".into(),
      HttpMethod::Head => "Head".into(),
      HttpMethod::Options => "Options".into(),
    }
  }

  fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
    Label::new(self.title()).text_color(self.color())
  }

  fn value(&self) -> &Self::Value {
    match self {
      HttpMethod::Get => &HttpMethod::Get,
      HttpMethod::Post => &HttpMethod::Post,
      HttpMethod::Put => &HttpMethod::Put,
      HttpMethod::Delete => &HttpMethod::Delete,
      HttpMethod::Patch => &HttpMethod::Patch,
      HttpMethod::Head => &HttpMethod::Head,
      HttpMethod::Options => &HttpMethod::Options,
    }
  }
}

/// Where a Rest param is placed in the outgoing request.
#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RestParamType {
  Path,
  Query,
  Header,
  Cookie,
}

impl SelectItem for RestParamType {
  type Value = RestParamType;

  fn title(&self) -> SharedString {
    self.into()
  }

  fn value(&self) -> &Self::Value {
    self
  }
}

impl From<&RestParamType> for SharedString {
  fn from(value: &RestParamType) -> Self {
    match value {
      RestParamType::Path => "Path",
      RestParamType::Query => "Query",
      RestParamType::Header => "Header",
      RestParamType::Cookie => "Cookie",
    }
      .into()
  }
}

/// Content type of raw body, serialized as its MIME string (e.g. `"application/json"`).
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum ContentType {
  ApplicationJson,
  /// A content type without dedicated support, sent as-is.
  Other(String),
}

impl ContentType {
  /// The MIME string of this content type.
  pub fn as_str(&self) -> &str {
    match self {
      ContentType::ApplicationJson => "application/json",
      ContentType::Other(content_type) => content_type,
    }
  }
}

impl From<String> for ContentType {
  /// Matches known MIME types case-insensitively, falling back to [`ContentType::Other`].
  fn from(content_type: String) -> Self {
    if content_type.eq_ignore_ascii_case("application/json") {
      ContentType::ApplicationJson
    } else {
      ContentType::Other(content_type)
    }
  }
}

impl Serialize for ContentType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for ContentType {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    String::deserialize(deserializer).map(ContentType::from)
  }
}
