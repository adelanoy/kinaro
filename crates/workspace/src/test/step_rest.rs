use crate::test::{TestsContainer, TestsContainerEvent};
use gpui_kit::component::Icon;
use gpui_kit::{Context, SharedString, WeakEntity};
use ki_project::{FileFormField, FileMultipartPart, FileRestBody, FileRestData, FileRestParam};
use ki_utils::EMPTY_SHARED_STRING;
use ki_utils::http::{
  CONTENT_TYPE_FORM_DATA, CONTENT_TYPE_FORM_URL_ENCODED, ContentType, HTTP_HEADER_CONTENT_TYPE, HttpMethod, RestParamType,
};
use log::warn;
use std::cmp::min;
use std::path::PathBuf;

/// Display name of [`RestBody::None`].
pub const REST_BODY_NONE: &str = "None";
/// Display name of [`RestBody::Raw`].
pub const REST_BODY_RAW: &str = "Raw";
/// Display name of [`RestBody::FormUrlEncoded`].
pub const REST_BODY_FORM_URL_ENCODED: &str = "Form URL Encoded";
/// Display name of [`RestBody::Multipart`].
pub const REST_BODY_MULTIPART: &str = "Multipart";

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
    /// The content-type of the body. Generates an ad-hoc 'Content-Type' header. Although, it can be superseded by user-made header.
    ///
    /// This is also used to set up the body's text editor.
    content_type: ContentType,
    /// Text sent in the request's body. It is split into lines when saved to file, to be diff-friendly.
    content: SharedString,
  },
  FormUrlEncoded {
    fields: Vec<FormField>,
  },
  Multipart {
    parts: Vec<MultipartPart>,
  },
}

impl RestBody {
  /// Display names of every body type, in the order they are offered to the user.
  ///
  /// Each name converts back to an empty body of its type through `RestBody::from(&str)`.
  pub const ALL: &'static [&str] = &[REST_BODY_NONE, REST_BODY_RAW, REST_BODY_FORM_URL_ENCODED, REST_BODY_MULTIPART];

  /// Display name of this body's type (one of [`RestBody::ALL`]), regardless of its content.
  pub fn type_as_str(&self) -> &'static str {
    match self {
      RestBody::None => REST_BODY_NONE,
      RestBody::Raw { .. } => REST_BODY_RAW,
      RestBody::FormUrlEncoded { .. } => REST_BODY_FORM_URL_ENCODED,
      RestBody::Multipart { .. } => REST_BODY_MULTIPART,
    }
  }
}

impl From<FileRestBody> for RestBody {
  fn from(body: FileRestBody) -> Self {
    match body {
      FileRestBody::None => RestBody::None,
      FileRestBody::Raw { content_type, lines } => RestBody::Raw {
        content_type,
        content: ki_utils::from_multiline(&lines).map(|s| s.into()).unwrap_or_default(),
      },
      FileRestBody::FormUrlEncoded { fields } => RestBody::FormUrlEncoded {
        fields: fields.into_iter().map(Into::into).collect(),
      },
      FileRestBody::Multipart { parts } => RestBody::Multipart {
        parts: parts.into_iter().map(Into::into).collect(),
      },
    }
  }
}

impl From<&RestBody> for FileRestBody {
  fn from(body: &RestBody) -> Self {
    match body {
      RestBody::None => FileRestBody::None,
      RestBody::Raw { content_type, content } => FileRestBody::Raw {
        content_type: *content_type,
        lines: ki_utils::to_multiline(content),
      },
      RestBody::FormUrlEncoded { fields } => FileRestBody::FormUrlEncoded {
        fields: fields.iter().map(Into::into).collect(),
      },
      RestBody::Multipart { parts } => FileRestBody::Multipart {
        parts: parts.iter().map(Into::into).collect(),
      },
    }
  }
}

impl From<&str> for RestBody {
  /// Builds an empty body from its type's display name (see [`RestBody::ALL`]), matched case-insensitively.
  ///
  /// A raw body starts as empty plain text. Unknown names fall back to [`RestBody::None`].
  fn from(value: &str) -> Self {
    if value.eq_ignore_ascii_case(REST_BODY_RAW) {
      RestBody::Raw {
        content_type: ContentType::Text,
        content: EMPTY_SHARED_STRING.clone(),
      }
    } else if value.eq_ignore_ascii_case(REST_BODY_MULTIPART) {
      RestBody::Multipart { parts: vec![] }
    } else if value.eq_ignore_ascii_case(REST_BODY_FORM_URL_ENCODED) {
      RestBody::FormUrlEncoded { fields: vec![] }
    } else {
      RestBody::None
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
    cx.notify();
  }

  /// The request body.
  #[inline]
  pub fn body(&self) -> &RestBody {
    &self.body
  }

  pub fn delete_param(&mut self, row_ix: usize, cx: &mut Context<Self>) {
    if row_ix >= self.params.len() {
      return;
    }
    self.params.remove(row_ix);
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    cx.notify();
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
    cx.notify();
  }

  #[inline]
  pub fn endpoint(&self) -> SharedString {
    self.endpoint.clone()
  }

  pub fn icon(&self) -> Icon {
    self.method.icon()
  }

  pub fn from_file(step: FileRestData, container: WeakEntity<TestsContainer>) -> Self {
    let FileRestData {
      method,
      endpoint,
      params,
      body,
    } = step;
    let body = body.into();
    let params: Vec<RestParam> = params.into_iter().map(Into::into).collect();

    let mut this = RestData {
      container,
      method,
      endpoint: endpoint.into(),
      params,
      body,
    };

    Self::update_generated_content_type_header(&mut this);
    this
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

  /// Replaces the body with `body` when it is of a different type than the current one.
  ///
  /// A body of the same type (e.g. a raw body replacing another raw body) is ignored, whatever its content:
  /// use [`RestData::set_body_raw_content`] or [`RestData::set_body_raw_content_type`] to edit a raw body.
  /// The generated `Content-Type` header is updated to match the new body (and removed for [`RestBody::None`]);
  /// user-made headers are left untouched.
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the body changed.
  pub fn set_body(&mut self, body: RestBody, cx: &mut Context<Self>) {
    use std::mem::discriminant;

    if discriminant(&body) == discriminant(&self.body) {
      return;
    }
    self.body = body;

    self.update_generated_content_type_header();
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    cx.notify();
  }

  /// Makes the generated `Content-Type` header match the body: updated in place if it exists, appended after
  /// the user params otherwise, and removed when the body has no content type.
  fn update_generated_content_type_header(&mut self) {
    let content_type_param = match &self.body {
      RestBody::Raw { content_type, .. } => Some(RestParam::from(content_type)),
      RestBody::None => None,
      RestBody::FormUrlEncoded { .. } => Some(RestParam {
        param_type: RestParamType::Header,
        name: HTTP_HEADER_CONTENT_TYPE.clone(),
        value: CONTENT_TYPE_FORM_URL_ENCODED.into(),
        disabled: false,
        generated: true,
      }),
      RestBody::Multipart { .. } => Some(RestParam {
        param_type: RestParamType::Header,
        name: HTTP_HEADER_CONTENT_TYPE.clone(),
        value: CONTENT_TYPE_FORM_DATA.into(),
        disabled: false,
        generated: true,
      }),
    };
    let existing_pos = self
      .params
      .iter()
      .position(|p| p.name == *HTTP_HEADER_CONTENT_TYPE && p.generated);
    match (content_type_param, existing_pos) {
      (Some(content_type), Some(pos)) => self.params[pos] = content_type,
      (Some(content_type), None) => self.params.push(content_type),
      (None, Some(pos)) => {
        self.params.remove(pos);
      }
      _ => {}
    }
  }

  /// Sets the text of a raw body. Does nothing if the body is not raw, or if the text is unchanged.
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the text changed.
  pub fn set_body_raw_content(&mut self, updated_content: SharedString, cx: &mut Context<Self>) {
    if let RestBody::Raw { content, .. } = &mut self.body
      && *content != updated_content
    {
      *content = updated_content;
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
      cx.notify();
    }
  }

  /// Sets the content type of a raw body, and updates the generated `Content-Type` header accordingly.
  /// Does nothing if the body is not raw, or if the content type is unchanged.
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the content type changed.
  pub fn set_body_raw_content_type(&mut self, updated_content_type: ContentType, cx: &mut Context<Self>) {
    if let RestBody::Raw { content_type, .. } = &mut self.body
      && *content_type != updated_content_type
    {
      *content_type = updated_content_type;
      self.update_generated_content_type_header();
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
      cx.notify();
    }
  }

  pub fn set_endpoint(&mut self, endpoint: SharedString, cx: &mut Context<Self>) {
    if self.endpoint != endpoint {
      self.endpoint = endpoint;
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
      cx.notify();
    }
  }

  pub fn set_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) -> bool {
    if self.method != method {
      self.method = method;
      _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
      cx.notify();
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
      cx.notify();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::test_fixtures::{load, record_events};
  use gpui_kit::{AppContext, Entity, TestAppContext};
  use ki_utils::http::{
    CONTENT_TYPE_FORM_DATA, CONTENT_TYPE_FORM_URL_ENCODED, CONTENT_TYPE_JSON, CONTENT_TYPE_PLAIN_TEXT, CONTENT_TYPE_XML,
  };

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
      (ContentType::Json, CONTENT_TYPE_JSON),
      (ContentType::Xml, CONTENT_TYPE_XML),
      (ContentType::Text, CONTENT_TYPE_PLAIN_TEXT),
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
  fn no_body_generates_no_params(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = RestData::from_file(file_rest(vec![], FileRestBody::None), tests.downgrade());
    assert!(rest.params().is_empty(), "unexpected params {:?}", rest.params());
  }

  #[gpui_kit::test]
  fn form_bodies_generate_their_content_type_header(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    for (body, mime) in [
      (FileRestBody::FormUrlEncoded { fields: vec![] }, CONTENT_TYPE_FORM_URL_ENCODED),
      (FileRestBody::Multipart { parts: vec![] }, CONTENT_TYPE_FORM_DATA),
    ] {
      let rest = RestData::from_file(file_rest(vec![], body), tests.downgrade());
      let [generated] = rest.params() else {
        panic!("expected only the generated Content-Type header, got {:?}", rest.params());
      };
      assert!(generated.generated());
      assert_eq!(generated.param_type, RestParamType::Header);
      assert_eq!(generated.name, *HTTP_HEADER_CONTENT_TYPE);
      assert_eq!(generated.value, mime);
      assert!(!generated.disabled);
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
    let file = file_rest(user_params, raw_body(ContentType::Json));
    let rest = RestData::from_file(file.clone(), tests.downgrade());

    assert_eq!(rest.params().iter().filter(|param| param.generated()).count(), 1);
    assert_eq!(rest.get_file(), file);
  }

  /// Loads `file` as a live REST step owned by `tests`.
  fn rest_entity(file: FileRestData, tests: &Entity<TestsContainer>, cx: &mut TestAppContext) -> Entity<RestData> {
    let container = tests.downgrade();
    cx.new(|_| RestData::from_file(file, container))
  }

  /// Values of the generated headers of `rest`.
  fn generated_headers(rest: &Entity<RestData>, cx: &TestAppContext) -> Vec<SharedString> {
    rest.read_with(cx, |rest, _| {
      rest
        .params()
        .iter()
        .filter(|p| p.generated())
        .map(|p| p.value.clone())
        .collect()
    })
  }

  fn raw(content_type: ContentType, content: &str) -> RestBody {
    RestBody::Raw {
      content_type,
      content: content.into(),
    }
  }

  #[test]
  fn body_type_names_convert_back_to_their_type() {
    assert_eq!(
      RestBody::ALL,
      [REST_BODY_NONE, REST_BODY_RAW, REST_BODY_FORM_URL_ENCODED, REST_BODY_MULTIPART]
    );
    for &name in RestBody::ALL {
      assert_eq!(RestBody::from(name).type_as_str(), name);
    }
  }

  #[test]
  fn type_as_str_ignores_the_content() {
    assert_eq!(raw(ContentType::Json, "{}").type_as_str(), REST_BODY_RAW);
    let fields = vec![FormField {
      name: "a".into(),
      value: "1".into(),
      disabled: false,
    }];
    assert_eq!(RestBody::FormUrlEncoded { fields }.type_as_str(), REST_BODY_FORM_URL_ENCODED);
  }

  #[test]
  fn body_from_name_builds_an_empty_body() {
    assert_eq!(RestBody::from(REST_BODY_RAW), raw(ContentType::Text, ""));
    assert_eq!(
      RestBody::from(REST_BODY_FORM_URL_ENCODED),
      RestBody::FormUrlEncoded { fields: vec![] }
    );
    assert_eq!(RestBody::from(REST_BODY_MULTIPART), RestBody::Multipart { parts: vec![] });
    assert_eq!(RestBody::from(REST_BODY_NONE), RestBody::None);
  }

  #[test]
  fn body_from_name_is_case_insensitive_and_falls_back_to_none() {
    assert_eq!(RestBody::from("raw"), raw(ContentType::Text, ""));
    assert_eq!(RestBody::from("MULTIPART"), RestBody::Multipart { parts: vec![] });
    assert_eq!(
      RestBody::from("form url encoded"),
      RestBody::FormUrlEncoded { fields: vec![] }
    );
    assert_eq!(RestBody::from("Binary"), RestBody::None);
    assert_eq!(RestBody::from(""), RestBody::None);
  }

  #[test]
  fn raw_body_lines_are_joined_and_split_back() {
    let file = FileRestBody::Raw {
      content_type: ContentType::Json,
      lines: vec!["{".into(), "  \"a\": 1".into(), "}".into(), "".into()],
    };
    let body = RestBody::from(file.clone());
    assert_eq!(body, raw(ContentType::Json, "{\n  \"a\": 1\n}\n"));
    assert_eq!(FileRestBody::from(&body), file);
  }

  #[test]
  fn empty_raw_body_is_saved_without_lines() {
    let file = FileRestBody::Raw {
      content_type: ContentType::Text,
      lines: vec![],
    };
    let body = RestBody::from(file.clone());
    assert_eq!(body, raw(ContentType::Text, ""));
    assert_eq!(FileRestBody::from(&body), file);
  }

  #[gpui_kit::test]
  fn body_returns_the_loaded_body(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = rest_entity(file_rest(vec![], raw_body(ContentType::Xml)), &tests, cx);
    assert_eq!(rest.read_with(cx, |rest, _| rest.body().clone()), raw(ContentType::Xml, "{}"));
  }

  #[gpui_kit::test]
  fn set_body_changes_the_type_and_its_generated_header(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let user_param = file_param(RestParamType::Query, "page", "1");
    let rest = rest_entity(file_rest(vec![user_param], FileRestBody::None), &tests, cx);
    let events = record_events(&tests, cx);

    for (body, expected_headers) in [
      (raw(ContentType::Json, ""), vec![CONTENT_TYPE_JSON]),
      (
        RestBody::FormUrlEncoded { fields: vec![] },
        vec![CONTENT_TYPE_FORM_URL_ENCODED],
      ),
      (RestBody::Multipart { parts: vec![] }, vec![CONTENT_TYPE_FORM_DATA]),
      (RestBody::None, vec![]),
    ] {
      rest.update(cx, |rest, cx| rest.set_body(body.clone(), cx));
      assert_eq!(rest.read_with(cx, |rest, _| rest.body().clone()), body);
      assert_eq!(generated_headers(&rest, cx), expected_headers);
      // The user param is kept, before the generated header
      assert_eq!(rest.read_with(cx, |rest, _| rest.params()[0].name.clone()), "page");
    }
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 4]);
  }

  #[gpui_kit::test]
  fn set_body_of_the_same_type_is_ignored(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = rest_entity(file_rest(vec![], raw_body(ContentType::Json)), &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.set_body(raw(ContentType::Xml, "other"), cx));
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.body().clone()),
      raw(ContentType::Json, "{}")
    );
    assert_eq!(generated_headers(&rest, cx), vec![CONTENT_TYPE_JSON]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn set_body_keeps_a_user_content_type_header(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let user_header = file_param(RestParamType::Header, "Content-Type", "application/xml");
    let rest = rest_entity(file_rest(vec![user_header.clone()], raw_body(ContentType::Json)), &tests, cx);

    rest.update(cx, |rest, cx| rest.set_body(RestBody::None, cx));
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.params().to_vec()),
      vec![RestParam::from(user_header)]
    );
  }

  #[gpui_kit::test]
  fn set_body_raw_content_notifies_changes_only(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = rest_entity(file_rest(vec![], raw_body(ContentType::Json)), &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.set_body_raw_content("{}".into(), cx));
    assert!(events.borrow().is_empty());

    rest.update(cx, |rest, cx| rest.set_body_raw_content("{\n}".into(), cx));
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.body().clone()),
      raw(ContentType::Json, "{\n}")
    );
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn set_body_raw_content_type_updates_the_generated_header(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = rest_entity(file_rest(vec![], raw_body(ContentType::Json)), &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.set_body_raw_content_type(ContentType::Json, cx));
    assert!(events.borrow().is_empty());

    rest.update(cx, |rest, cx| rest.set_body_raw_content_type(ContentType::Xml, cx));
    assert_eq!(rest.read_with(cx, |rest, _| rest.body().clone()), raw(ContentType::Xml, "{}"));
    assert_eq!(generated_headers(&rest, cx), vec![CONTENT_TYPE_XML]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn raw_body_setters_ignore_other_bodies(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = rest_entity(file_rest(vec![], FileRestBody::Multipart { parts: vec![] }), &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| {
      rest.set_body_raw_content("text".into(), cx);
      rest.set_body_raw_content_type(ContentType::Json, cx);
    });
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.body().clone()),
      RestBody::Multipart { parts: vec![] }
    );
    assert_eq!(generated_headers(&rest, cx), vec![CONTENT_TYPE_FORM_DATA]);
    assert!(events.borrow().is_empty());
  }
}
