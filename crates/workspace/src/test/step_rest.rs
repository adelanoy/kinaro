use crate::test::{TestsContainer, TestsContainerEvent};
use gpui_kit::component::Icon;
use gpui_kit::{Context, SharedString, Styled, WeakEntity};
use ki_assets::icon::IconAsset;
use ki_project::{FileFormField, FileMultipartPart, FileRestBody, FileRestData, FileRestParam};
use ki_utils::http::{ContentType, HTTP_HEADER_CONTENT_TYPE, HttpMethod, RestParamType};
use log::warn;
use std::cmp::min;
use std::path::PathBuf;

/// A name/value pair sent as part of the request.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct RestParam {
  pub param_type: RestParamType,
  pub name: SharedString,
  pub value: SharedString,
  pub disabled: bool,
  generated: bool,
}

impl RestParam {
  #[inline]
  pub fn generated(&self) -> bool {
    self.generated
  }
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
      param_type,
      name: name.into(),
      value: value.into(),
      disabled,
      generated: false,
    }
  }
}

impl From<&RestParam> for FileRestParam {
  fn from(param: &RestParam) -> Self {
    FileRestParam {
      param_type: param.param_type,
      name: param.name.to_string(),
      value: param.value.to_string(),
      disabled: param.disabled,
    }
  }
}

impl From<&ContentType> for RestParam {
  fn from(value: &ContentType) -> Self {
    Self {
      param_type: RestParamType::Header,
      name: HTTP_HEADER_CONTENT_TYPE.clone(),
      value: value.as_str().into(),
      disabled: false,
      generated: true,
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
    content_type: ContentType,
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
        content_type,
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
        content_type: content_type.clone(),
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
  pub(crate) method: HttpMethod,
  pub(crate) endpoint: SharedString,
  pub(crate) params: Vec<RestParam>,
  pub(crate) body: RestBody,
}

impl RestData {
  pub fn add_param(&mut self, row_ix: Option<usize>, cx: &mut Context<Self>) {
    let ix = row_ix.map(|ix| min(ix + 1, self.params.len())).unwrap_or(self.params.len());
    let name = ki_utils::next_available_name("Param", self.params.iter().map(|param| param.name.clone()));
    let param = RestParam {
      param_type: RestParamType::Query,
      name,
      value: Default::default(),
      disabled: false,
      generated: false,
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
      HttpMethod::Get => Icon::new(IconAsset::StepRestGet).text_color(color),
      HttpMethod::Post => Icon::new(IconAsset::StepRestPost).text_color(color),
      HttpMethod::Put => Icon::new(IconAsset::StepRestPut).text_color(color),
      HttpMethod::Delete => Icon::new(IconAsset::StepRestDelete).text_color(color),
      HttpMethod::Patch => Icon::new(IconAsset::StepRestPatch).text_color(color),
      HttpMethod::Head => Icon::new(IconAsset::StepRestHead).text_color(color),
      HttpMethod::Options => Icon::new(IconAsset::StepRestOptions).text_color(color),
    }
  }

  pub fn from_file(step: FileRestData, container: WeakEntity<TestsContainer>) -> Self {
    let FileRestData {
      method,
      endpoint,
      params,
      body,
    } = step;
    let body = body.into();
    // Generated params
    let mut params: Vec<RestParam> = params.into_iter().map(Into::into).collect();
    if let RestBody::Raw { content_type, .. } = &body {
      params.push(RestParam::from(content_type))
    }
    RestData {
      container,
      method,
      endpoint: endpoint.into(),
      params,
      body,
    }
  }

  pub fn get_file(&self) -> FileRestData {
    FileRestData {
      method: self.method,
      endpoint: self.endpoint.to_string(),
      params: self.params.iter().filter(|p| !p.generated).map(Into::into).collect(),
      body: (&self.body).into(),
    }
  }

  #[inline]
  pub fn method(&self) -> HttpMethod {
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

  pub fn set_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) -> bool {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::test_fixtures::load;
  use gpui_kit::TestAppContext;

  fn file_param(param_type: RestParamType, name: &str, value: &str) -> FileRestParam {
    FileRestParam {
      param_type,
      name: name.into(),
      value: value.into(),
      disabled: false,
    }
  }

  fn file_rest(params: Vec<FileRestParam>, body: FileRestBody) -> FileRestData {
    FileRestData {
      method: HttpMethod::Post,
      endpoint: "/items".into(),
      params,
      body,
    }
  }

  fn raw_body(content_type: ContentType) -> FileRestBody {
    FileRestBody::Raw {
      content_type,
      lines: vec!["{}".into()],
    }
  }

  #[gpui_kit::test]
  fn raw_body_generates_a_content_type_header_after_user_params(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    for (content_type, mime) in [
      (ContentType::ApplicationJson, "application/json"),
      (ContentType::Other("text/csv".into()), "text/csv"),
    ] {
      let file = file_rest(vec![file_param(RestParamType::Query, "page", "1")], raw_body(content_type));
      let rest = RestData::from_file(file, tests.downgrade());

      let [user, generated] = rest.params() else {
        panic!(
          "expected the user param followed by the generated one, got {:?}",
          rest.params()
        );
      };
      assert!(!user.generated());
      assert!(generated.generated());
      assert_eq!(generated.param_type, RestParamType::Header);
      assert_eq!(generated.name, *HTTP_HEADER_CONTENT_TYPE);
      assert_eq!(generated.value, mime);
      assert!(!generated.disabled);
    }
  }

  #[gpui_kit::test]
  fn non_raw_bodies_generate_no_params(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    for body in [
      FileRestBody::None,
      FileRestBody::FormUrlEncoded { fields: vec![] },
      FileRestBody::Multipart { parts: vec![] },
      FileRestBody::Binary { path: "blob.bin".into() },
    ] {
      let rest = RestData::from_file(file_rest(vec![], body), tests.downgrade());
      assert!(rest.params().is_empty(), "unexpected params {:?}", rest.params());
    }
  }

  #[gpui_kit::test]
  fn get_file_saves_only_user_params(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    // A user-defined Content-Type header must be kept, unlike the generated one
    let user_params = vec![
      file_param(RestParamType::Query, "page", "1"),
      file_param(RestParamType::Header, "Content-Type", "application/xml"),
    ];
    let file = file_rest(user_params, raw_body(ContentType::ApplicationJson));
    let rest = RestData::from_file(file.clone(), tests.downgrade());

    assert_eq!(rest.params().iter().filter(|param| param.generated()).count(), 1);
    assert_eq!(rest.get_file(), file);
  }
}
