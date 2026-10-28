use crate::test::{TestsContainer, TestsContainerEvent};
use gpui_kit::component::Icon;
use gpui_kit::component::label::Label;
use gpui_kit::component::select::SelectItem;
use gpui_kit::{App, Context, Hsla, IntoElement, Rgba, SharedString, Styled, WeakEntity, Window};
use ki_assets::icon::IconAsset;
use ki_project::{
  FileFormField, FileMultipartPart, FileRestBody, FileRestData, FileRestMethod, FileRestParam, FileRestParamType,
};
use log::warn;
use std::cmp::min;
use std::path::PathBuf;

const REST_GET_COLOR: &str = "#38bdf8";
const REST_POST_COLOR: &str = "#fcd34d";
const REST_PUT_COLOR: &str = "#b45309";
const REST_DELETE_COLOR: &str = "#ef4444";
const REST_PATCH_COLOR: &str = "#9333ea";
const REST_OPTIONS_COLOR: &str = "#4ade80";
const REST_HEAD_COLOR: &str = "#ec4899";

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum RestMethod {
  Get,
  Post,
  Put,
  Delete,
  Patch,
  Head,
  Options,
}

impl RestMethod {
  pub const ALL: &'static [RestMethod] = &[
    RestMethod::Get,
    RestMethod::Post,
    RestMethod::Put,
    RestMethod::Delete,
    RestMethod::Patch,
  ];

  pub fn color(&self) -> impl Into<Hsla> {
    let hex_str = match self {
      RestMethod::Get => REST_GET_COLOR,
      RestMethod::Post => REST_POST_COLOR,
      RestMethod::Put => REST_PUT_COLOR,
      RestMethod::Delete => REST_DELETE_COLOR,
      RestMethod::Patch => REST_PATCH_COLOR,
      RestMethod::Head => REST_OPTIONS_COLOR,
      RestMethod::Options => REST_HEAD_COLOR,
    };
    Rgba::try_from(hex_str).unwrap()
  }
}

impl SelectItem for RestMethod {
  type Value = RestMethod;

  fn title(&self) -> SharedString {
    match self {
      RestMethod::Get => "Get".into(),
      RestMethod::Post => "Post".into(),
      RestMethod::Put => "Put".into(),
      RestMethod::Delete => "Delete".into(),
      RestMethod::Patch => "Patch".into(),
      RestMethod::Head => "Head".into(),
      RestMethod::Options => "Options".into(),
    }
  }

  fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
    Label::new(self.title()).text_color(self.color())
  }

  fn value(&self) -> &Self::Value {
    match self {
      RestMethod::Get => &RestMethod::Get,
      RestMethod::Post => &RestMethod::Post,
      RestMethod::Put => &RestMethod::Put,
      RestMethod::Delete => &RestMethod::Delete,
      RestMethod::Patch => &RestMethod::Patch,
      RestMethod::Head => &RestMethod::Head,
      RestMethod::Options => &RestMethod::Options,
    }
  }
}

impl From<FileRestMethod> for RestMethod {
  fn from(method: FileRestMethod) -> Self {
    match method {
      FileRestMethod::Get => RestMethod::Get,
      FileRestMethod::Post => RestMethod::Post,
      FileRestMethod::Put => RestMethod::Put,
      FileRestMethod::Delete => RestMethod::Delete,
      FileRestMethod::Patch => RestMethod::Patch,
      FileRestMethod::Head => RestMethod::Head,
      FileRestMethod::Options => RestMethod::Options,
    }
  }
}

impl From<&RestMethod> for FileRestMethod {
  fn from(method: &RestMethod) -> Self {
    match method {
      RestMethod::Get => FileRestMethod::Get,
      RestMethod::Post => FileRestMethod::Post,
      RestMethod::Put => FileRestMethod::Put,
      RestMethod::Delete => FileRestMethod::Delete,
      RestMethod::Patch => FileRestMethod::Patch,
      RestMethod::Head => FileRestMethod::Head,
      RestMethod::Options => FileRestMethod::Options,
    }
  }
}

/// Where a [`RestParam`] is placed in the outgoing request.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum RestParamType {
  Path,
  Query,
  Header,
  Cookie,
}

impl From<FileRestParamType> for RestParamType {
  fn from(param_type: FileRestParamType) -> Self {
    match param_type {
      FileRestParamType::Path => RestParamType::Path,
      FileRestParamType::Query => RestParamType::Query,
      FileRestParamType::Header => RestParamType::Header,
      FileRestParamType::Cookie => RestParamType::Cookie,
    }
  }
}

impl From<&RestParamType> for FileRestParamType {
  fn from(param_type: &RestParamType) -> Self {
    match param_type {
      RestParamType::Path => FileRestParamType::Path,
      RestParamType::Query => FileRestParamType::Query,
      RestParamType::Header => FileRestParamType::Header,
      RestParamType::Cookie => FileRestParamType::Cookie,
    }
  }
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

/// A name/value pair sent as part of the request.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct RestParam {
  pub param_type: RestParamType,
  pub name: SharedString,
  pub value: SharedString,
  pub disabled: bool,
}

impl From<FileRestParam> for RestParam {
  fn from(param: FileRestParam) -> Self {
    let FileRestParam {
      param_type,
      name,
      value,
      disabled,
    } = param;
    RestParam {
      param_type: param_type.into(),
      name: name.into(),
      value: value.into(),
      disabled,
    }
  }
}

impl From<&RestParam> for FileRestParam {
  fn from(param: &RestParam) -> Self {
    FileRestParam {
      param_type: (&param.param_type).into(),
      name: param.name.to_string(),
      value: param.value.to_string(),
      disabled: param.disabled,
    }
  }
}

/// A field of a `application/x-www-form-urlencoded` body.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct FormField {
  pub name: SharedString,
  pub value: SharedString,
  pub disabled: bool,
}

impl From<FileFormField> for FormField {
  fn from(field: FileFormField) -> Self {
    let FileFormField { name, value, disabled } = field;
    FormField {
      name: name.into(),
      value: value.into(),
      disabled,
    }
  }
}

impl From<&FormField> for FileFormField {
  fn from(field: &FormField) -> Self {
    FileFormField {
      name: field.name.to_string(),
      value: field.value.to_string(),
      disabled: field.disabled,
    }
  }
}

/// A part of a `multipart/form-data` body.
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MultipartPart {
  Text {
    name: SharedString,
    value: SharedString,
    disabled: bool,
  },
  File {
    name: SharedString,
    path: PathBuf,
    content_type: Option<SharedString>,
    disabled: bool,
  },
}

impl From<FileMultipartPart> for MultipartPart {
  fn from(part: FileMultipartPart) -> Self {
    match part {
      FileMultipartPart::Text { name, value, disabled } => MultipartPart::Text {
        name: name.into(),
        value: value.into(),
        disabled,
      },
      FileMultipartPart::File {
        name,
        path,
        content_type,
        disabled,
      } => MultipartPart::File {
        name: name.into(),
        path,
        content_type: content_type.map(Into::into),
        disabled,
      },
    }
  }
}

impl From<&MultipartPart> for FileMultipartPart {
  fn from(part: &MultipartPart) -> Self {
    match part {
      MultipartPart::Text { name, value, disabled } => FileMultipartPart::Text {
        name: name.to_string(),
        value: value.to_string(),
        disabled: *disabled,
      },
      MultipartPart::File {
        name,
        path,
        content_type,
        disabled,
      } => FileMultipartPart::File {
        name: name.to_string(),
        path: path.to_owned(),
        content_type: content_type.as_ref().map(|v| v.to_string()),
        disabled: *disabled,
      },
    }
  }
}

/// The request body. Its variant determines the `Content-Type` sent with the request.
#[derive(Eq, PartialEq, Clone, Debug, Default)]
pub enum RestBody {
  #[default]
  None,
  /// Free text (JSON, XML, plain text...) sent as-is.
  Raw {
    content_type: SharedString,
    /// One entry per line, so version control diffs only the lines that
    /// actually changed instead of one escaped multiline string.
    lines: Vec<SharedString>,
  },
  FormUrlEncoded {
    fields: Vec<FormField>,
  },
  Multipart {
    parts: Vec<MultipartPart>,
  },
  /// The raw bytes of a single file.
  Binary {
    path: PathBuf,
  },
}

impl From<FileRestBody> for RestBody {
  fn from(body: FileRestBody) -> Self {
    match body {
      FileRestBody::None => RestBody::None,
      FileRestBody::Raw { content_type, lines } => RestBody::Raw {
        content_type: content_type.into(),
        lines: lines.into_iter().map(Into::into).collect(),
      },
      FileRestBody::FormUrlEncoded { fields } => RestBody::FormUrlEncoded {
        fields: fields.into_iter().map(Into::into).collect(),
      },
      FileRestBody::Multipart { parts } => RestBody::Multipart {
        parts: parts.into_iter().map(Into::into).collect(),
      },
      FileRestBody::Binary { path } => RestBody::Binary { path },
    }
  }
}

impl From<&RestBody> for FileRestBody {
  fn from(body: &RestBody) -> Self {
    match body {
      RestBody::None => FileRestBody::None,
      RestBody::Raw { content_type, lines } => FileRestBody::Raw {
        content_type: content_type.to_string(),
        lines: lines.iter().map(|line| line.to_string()).collect(),
      },
      RestBody::FormUrlEncoded { fields } => FileRestBody::FormUrlEncoded {
        fields: fields.iter().map(Into::into).collect(),
      },
      RestBody::Multipart { parts } => FileRestBody::Multipart {
        parts: parts.iter().map(Into::into).collect(),
      },
      RestBody::Binary { path } => FileRestBody::Binary { path: path.to_owned() },
    }
  }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct RestData {
  /// The container holding this step, notified of the changes made to the step
  pub(crate) container: WeakEntity<TestsContainer>,
  pub(crate) method: RestMethod,
  pub(crate) endpoint: SharedString,
  pub(crate) params: Vec<RestParam>,
  pub(crate) body: RestBody,
}

impl RestData {
  pub fn add_param(&mut self, row_ix: Option<usize>, cx: &mut Context<Self>) {
    let ix = row_ix
      .map(|ix| min(ix + 1, self.params.len()))
      .unwrap_or(self.params.len());
    let name = ki_utils::next_available_name("Param", self.params.iter().map(|param| param.name.clone()));
    let param = RestParam {
      param_type: RestParamType::Query,
      name,
      value: Default::default(),
      disabled: false,
    };
    self.params.insert(ix, param);
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
  }

  pub fn delete_param(&mut self, row_ix: usize, cx: &mut Context<Self>) {
    if row_ix >= self.params.len() {
      return;
    }
    self.params.remove(row_ix);
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
  }

  pub fn duplicate_param(&mut self, row_ix: usize, cx: &mut Context<Self>) {
    let Some(mut param) = self.params.get(row_ix).cloned() else {
      return;
    };
    let name = ki_utils::next_available_name(&param.name, self.params.iter().map(|param| param.name.clone()));
    param.name = name;
    let ix = min(row_ix + 1, self.params.len());
    self.params.insert(ix, param);
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
  }

  #[inline]
  pub fn endpoint(&self) -> SharedString {
    self.endpoint.clone()
  }

  pub fn icon(&self) -> Icon {
    let color = self.method.color();
    match self.method {
      RestMethod::Get => Icon::new(IconAsset::StepRestGet).text_color(color),
      RestMethod::Post => Icon::new(IconAsset::StepRestPost).text_color(color),
      RestMethod::Put => Icon::new(IconAsset::StepRestPut).text_color(color),
      RestMethod::Delete => Icon::new(IconAsset::StepRestDelete).text_color(color),
      RestMethod::Patch => Icon::new(IconAsset::StepRestPatch).text_color(color),
      RestMethod::Head => Icon::new(IconAsset::StepRestHead).text_color(color),
      RestMethod::Options => Icon::new(IconAsset::StepRestOptions).text_color(color),
    }
  }

  pub fn from_file(step: FileRestData, container: WeakEntity<TestsContainer>) -> Self {
    let FileRestData {
      method,
      endpoint,
      params,
      body,
    } = step;
    RestData {
      container,
      method: method.into(),
      endpoint: endpoint.into(),
      params: params.into_iter().map(Into::into).collect(),
      body: body.into(),
    }
  }

  pub fn get_file(&self) -> FileRestData {
    FileRestData {
      method: (&self.method).into(),
      endpoint: self.endpoint.to_string(),
      params: self.params.iter().map(Into::into).collect(),
      body: (&self.body).into(),
    }
  }

  #[inline]
  pub fn method(&self) -> RestMethod {
    self.method
  }

  #[inline]
  pub fn params(&self) -> &[RestParam] {
    &self.params
  }

  pub fn set_endpoint(&mut self, endpoint: SharedString, cx: &mut Context<Self>) {
    if self.endpoint != endpoint {
      self.endpoint = endpoint;
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    }
  }

  pub fn set_method(&mut self, method: RestMethod, cx: &mut Context<Self>) -> bool {
    if self.method != method {
      self.method = method;
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
      true
    } else {
      false
    }
  }

  pub fn update_params(&mut self, row_ix: usize, param: RestParam, cx: &mut Context<Self>) {
    if row_ix >= self.params.len() {
      warn!(
        "RestParams::update_params: Attempted to update a non-existent index: {}",
        row_ix
      );
      return;
    }
    if self.params[row_ix] != param {
      self.params[row_ix] = param;
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    }
  }
}
