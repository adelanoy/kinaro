use gpui_kit::component::Icon;
use gpui_kit::component::label::Label;
use gpui_kit::component::select::SelectItem;
use gpui_kit::{App, Hsla, IntoElement, Rgba, SharedString, Styled, Window};
use ki_assets::icon::IconAsset;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::LazyLock;

/// Default endpoint of a new project.
pub static LOCALHOST_ENDPOINT: LazyLock<SharedString> = LazyLock::new(|| SharedString::new("http://localhost:8080"));
/// Name of the HTTP header giving the media type of the request body.
pub static HTTP_HEADER_CONTENT_TYPE: LazyLock<SharedString> = LazyLock::new(|| SharedString::new("Content-Type"));
/// MIME type of a `multipart/form-data` body.
pub const CONTENT_TYPE_FORM_DATA: &str = "multipart/form-data";
/// MIME type of a URL-encoded form body.
pub const CONTENT_TYPE_FORM_URL_ENCODED: &str = "application/x-www-form-urlencoded";
/// MIME type of [`ContentType::Json`].
pub const CONTENT_TYPE_JSON: &str = "application/json";
/// Display name of [`ContentType::Json`].
pub const CONTENT_TYPE_JSON_LABEL: &str = "JSON";
/// MIME type of [`ContentType::Text`].
pub const CONTENT_TYPE_PLAIN_TEXT: &str = "text/plain";
/// Display name of [`ContentType::Text`].
pub const CONTENT_TYPE_PLAIN_TEXT_LABEL: &str = "Text";
/// MIME type of [`ContentType::Xml`].
pub const CONTENT_TYPE_XML: &str = "application/xml";
/// Display name of [`ContentType::Xml`].
pub const CONTENT_TYPE_XML_LABEL: &str = "XML";
/// MIME type of [`ContentType::Csv`].
pub const CONTENT_TYPE_CSV: &str = "text/csv";
/// Display name of [`ContentType::Csv`].
pub const CONTENT_TYPE_CSV_LABEL: &str = "CSV";
/// MIME type of [`ContentType::Yaml`].
pub const CONTENT_TYPE_YAML: &str = "application/yaml";
/// Display name of [`ContentType::Yaml`].
pub const CONTENT_TYPE_YAML_LABEL: &str = "YAML";
const REST_GET_COLOR: &str = "#38bdf8";
const REST_POST_COLOR: &str = "#fcd34d";
const REST_PUT_COLOR: &str = "#b45309";
const REST_DELETE_COLOR: &str = "#ef4444";
const REST_PATCH_COLOR: &str = "#9333ea";
const REST_OPTIONS_COLOR: &str = "#4ade80";
const REST_HEAD_COLOR: &str = "#ec4899";

/// Method of a REST request, serialized as its uppercase name (e.g. `"GET"`).
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
  /// Every method, in the order they are offered to the user.
  pub const ALL: &'static [HttpMethod] = &[
    HttpMethod::Get,
    HttpMethod::Post,
    HttpMethod::Put,
    HttpMethod::Delete,
    HttpMethod::Patch,
    HttpMethod::Head,
    HttpMethod::Options,
  ];

  /// The method's name, as sent in a request (`GET`, `POST`, ...).
  pub fn as_str(&self) -> &'static str {
    match self {
      HttpMethod::Get => "GET",
      HttpMethod::Post => "POST",
      HttpMethod::Put => "PUT",
      HttpMethod::Delete => "DELETE",
      HttpMethod::Patch => "PATCH",
      HttpMethod::Head => "HEAD",
      HttpMethod::Options => "OPTIONS",
    }
  }

  /// The color identifying this method in the UI (its label and its step icon).
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

  /// The icon of a REST step using this method, tinted with [`HttpMethod::color`].
  pub fn icon(&self) -> Icon {
    match self {
      HttpMethod::Get => Icon::new(IconAsset::StepRestGet).text_color(self.color()),
      HttpMethod::Post => Icon::new(IconAsset::StepRestPost).text_color(self.color()),
      HttpMethod::Put => Icon::new(IconAsset::StepRestPut).text_color(self.color()),
      HttpMethod::Delete => Icon::new(IconAsset::StepRestDelete).text_color(self.color()),
      HttpMethod::Patch => Icon::new(IconAsset::StepRestPatch).text_color(self.color()),
      HttpMethod::Head => Icon::new(IconAsset::StepRestHead).text_color(self.color()),
      HttpMethod::Options => Icon::new(IconAsset::StepRestOptions).text_color(self.color()),
    }
  }
}

impl SelectItem for HttpMethod {
  type Value = HttpMethod;

  fn title(&self) -> SharedString {
    self.as_str().into()
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

/// Where a Rest param is placed in the outgoing request, serialized in camelCase (e.g. `"query"`).
#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RestParamType {
  /// Substituted in the endpoint's path.
  Path,
  /// Appended to the URL's query string.
  Query,
  /// Sent as an HTTP header.
  Header,
  /// Sent in the `Cookie` header.
  Cookie,
}

impl RestParamType {
  pub const ALL: &'static [RestParamType] = &[
    RestParamType::Path,
    RestParamType::Query,
    RestParamType::Header,
    RestParamType::Cookie,
  ];
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
  /// The display name of the param type (e.g. `"Query"`).
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
///
/// Deserializing an unknown MIME string gives [`ContentType::Text`] (see `From<String>`).
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum ContentType {
  /// `application/json`
  Json,
  /// `application/xml`
  Xml,
  /// `text/plain`, also the fallback for unknown MIME types.
  Text,
  /// `text/csv`
  Csv,
  /// `application/yaml`
  Yaml,
}

impl ContentType {
  /// Every content type, in the order they are offered to the user.
  pub const ALL: &'static [ContentType] = &[
    ContentType::Json,
    ContentType::Xml,
    ContentType::Text,
    ContentType::Csv,
    ContentType::Yaml,
  ];
  /// The MIME string of this content type.
  pub fn as_str(&self) -> &str {
    match self {
      ContentType::Json => CONTENT_TYPE_JSON,
      ContentType::Xml => CONTENT_TYPE_XML,
      ContentType::Text => CONTENT_TYPE_PLAIN_TEXT,
      ContentType::Csv => CONTENT_TYPE_CSV,
      ContentType::Yaml => CONTENT_TYPE_YAML,
    }
  }
}

impl From<String> for ContentType {
  /// Matches known MIME types case-insensitively, falling back to [`ContentType::Text`]
  /// for any other string (including form MIME types, which are not raw body types).
  fn from(content_type: String) -> Self {
    if content_type.eq_ignore_ascii_case(CONTENT_TYPE_JSON) {
      ContentType::Json
    } else if content_type.eq_ignore_ascii_case(CONTENT_TYPE_XML) {
      ContentType::Xml
    } else if content_type.eq_ignore_ascii_case(CONTENT_TYPE_CSV) {
      ContentType::Csv
    } else if content_type.eq_ignore_ascii_case(CONTENT_TYPE_YAML) {
      ContentType::Yaml
    } else {
      ContentType::Text
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

impl SelectItem for ContentType {
  type Value = Self;

  /// The display name of the content type (e.g. `"JSON"`), not its MIME string.
  fn title(&self) -> SharedString {
    match self {
      ContentType::Json => CONTENT_TYPE_JSON_LABEL,
      ContentType::Xml => CONTENT_TYPE_XML_LABEL,
      ContentType::Text => CONTENT_TYPE_PLAIN_TEXT_LABEL,
      ContentType::Csv => CONTENT_TYPE_CSV_LABEL,
      ContentType::Yaml => CONTENT_TYPE_YAML_LABEL,
    }
    .into()
  }

  fn value(&self) -> &Self::Value {
    self
  }
}

#[cfg(test)]
mod tests {
  use crate::http::*;
  use gpui_kit::private::serde_json;
  use gpui_kit::private::serde_json::json;
  use std::collections::HashSet;

  const METHODS: [(HttpMethod, &str); 7] = [
    (HttpMethod::Get, "GET"),
    (HttpMethod::Post, "POST"),
    (HttpMethod::Put, "PUT"),
    (HttpMethod::Delete, "DELETE"),
    (HttpMethod::Patch, "PATCH"),
    (HttpMethod::Head, "HEAD"),
    (HttpMethod::Options, "OPTIONS"),
  ];

  const CONTENT_TYPES: [(ContentType, &str, &str); 5] = [
    (ContentType::Json, CONTENT_TYPE_JSON, CONTENT_TYPE_JSON_LABEL),
    (ContentType::Xml, CONTENT_TYPE_XML, CONTENT_TYPE_XML_LABEL),
    (ContentType::Text, CONTENT_TYPE_PLAIN_TEXT, CONTENT_TYPE_PLAIN_TEXT_LABEL),
    (ContentType::Csv, CONTENT_TYPE_CSV, CONTENT_TYPE_CSV_LABEL),
    (ContentType::Yaml, CONTENT_TYPE_YAML, CONTENT_TYPE_YAML_LABEL),
  ];

  #[test]
  fn http_method_all_lists_every_method_once() {
    assert_eq!(HttpMethod::ALL, METHODS.map(|(method, _)| method));
  }

  #[test]
  fn http_method_serializes_as_its_uppercase_name() {
    for (method, name) in METHODS {
      assert_eq!(method.as_str(), name);
      assert_eq!(serde_json::to_value(method).unwrap(), json!(name));
      assert_eq!(serde_json::from_value::<HttpMethod>(json!(name)).unwrap(), method);
    }
  }

  #[test]
  fn http_method_rejects_unknown_names() {
    assert!(serde_json::from_value::<HttpMethod>(json!("get")).is_err());
    assert!(serde_json::from_value::<HttpMethod>(json!("TRACE")).is_err());
  }

  #[test]
  fn http_method_select_item_is_its_name() {
    for (method, name) in METHODS {
      assert_eq!(method.title(), name);
      assert_eq!(*method.value(), method);
    }
  }

  #[test]
  fn http_method_colors_are_valid_and_distinct() {
    let colors: HashSet<_> = HttpMethod::ALL
      .iter()
      .map(|method| {
        let color: Hsla = method.color().into();
        format!("{color:?}")
      })
      .collect();
    assert_eq!(colors.len(), HttpMethod::ALL.len());
  }

  #[test]
  fn rest_param_type_serializes_in_camel_case() {
    for (param_type, name, label) in [
      (RestParamType::Path, "path", "Path"),
      (RestParamType::Query, "query", "Query"),
      (RestParamType::Header, "header", "Header"),
      (RestParamType::Cookie, "cookie", "Cookie"),
    ] {
      assert_eq!(serde_json::to_value(param_type).unwrap(), json!(name));
      assert_eq!(serde_json::from_value::<RestParamType>(json!(name)).unwrap(), param_type);
      assert_eq!(SharedString::from(&param_type), label);
      assert_eq!(param_type.title(), label);
      assert_eq!(*param_type.value(), param_type);
    }
  }

  #[test]
  fn content_type_all_lists_every_type_once() {
    assert_eq!(ContentType::ALL, CONTENT_TYPES.map(|(content_type, _, _)| content_type));
  }

  #[test]
  fn content_type_serializes_as_mime_string() {
    for (content_type, mime, _) in CONTENT_TYPES {
      assert_eq!(content_type.as_str(), mime);
      assert_eq!(serde_json::to_value(content_type).unwrap(), json!(mime));
      assert_eq!(serde_json::from_value::<ContentType>(json!(mime)).unwrap(), content_type);
    }
  }

  #[test]
  fn content_type_is_matched_case_insensitively() {
    for (content_type, mime, _) in CONTENT_TYPES {
      assert_eq!(ContentType::from(mime.to_uppercase()), content_type);
    }
    assert_eq!(ContentType::from("Application/JSON".to_string()), ContentType::Json);
  }

  #[test]
  fn unknown_content_type_falls_back_to_text() {
    for mime in [CONTENT_TYPE_FORM_DATA, CONTENT_TYPE_FORM_URL_ENCODED, "image/png", ""] {
      assert_eq!(ContentType::from(mime.to_string()), ContentType::Text, "{mime:?}");
      assert_eq!(serde_json::from_value::<ContentType>(json!(mime)).unwrap(), ContentType::Text);
    }
  }

  #[test]
  fn content_type_rejects_non_string_values() {
    assert!(serde_json::from_value::<ContentType>(json!(1)).is_err());
    assert!(serde_json::from_value::<ContentType>(json!(null)).is_err());
  }

  #[test]
  fn content_type_select_item_is_its_label() {
    for (content_type, _, label) in CONTENT_TYPES {
      assert_eq!(content_type.title(), label);
      assert_eq!(*content_type.value(), content_type);
    }
  }
}
