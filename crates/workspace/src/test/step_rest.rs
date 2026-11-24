use crate::TestStep;
use crate::test::test_step::{StepData, StepDataEvent};
use crate::test::{TestsContainer, TestsContainerEvent};
use gpui_kit::component::Icon;
use gpui_kit::{Context, EventEmitter, SharedString, WeakEntity};
use ki_project::{FileFormField, FileMultipartPart, FileRestBody, FileRestData, FileRestParam};
use ki_utils::EMPTY_SHARED_STRING;
use ki_utils::http::{
  CONTENT_TYPE_FORM_DATA, CONTENT_TYPE_FORM_URL_ENCODED, ContentType, HTTP_HEADER_CONTENT_TYPE, HttpMethod, RestParamType,
};
use ki_utils::undo::Edit;
use log::error;
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
  pub const ALL: &'static [&str] = &[REST_BODY_NONE, REST_BODY_RAW, REST_BODY_FORM_URL_ENCODED];

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

/// A step sending a REST request.
///
/// Not `Clone`: a copy must be attached to its own step, see [`RestData::duplicate`].
#[derive(Eq, PartialEq, Debug)]
pub struct RestData {
  /// The container holding this step, notified of the changes made to the step
  pub(crate) container: WeakEntity<TestsContainer>,
  /// The parent step, to allow pushing undo/redo edits
  pub parent: WeakEntity<TestStep>,
  pub(crate) method: HttpMethod,
  pub(crate) endpoint: SharedString,
  pub(crate) params: Vec<RestParam>,
  pub(crate) body: RestBody,
  /// State of the request when the raw-content edit in progress started, if any (see
  /// [`RestData::begin_raw_content_edit`])
  pub(crate) raw_edit_start: Option<RestState>,
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
    self.record(cx, |this| this.params.insert(ix, param));
  }

  /// Inserts a new, empty and enabled field in a URL-encoded body, right after the field at `row_ix`, or at the end
  /// when `row_ix` is `None` or past the last field.
  ///
  /// The field is named `Param`, suffixed (`Param_1`, `Param_2`...) when that name is already used by another field.
  /// Does nothing if the body is not URL-encoded.
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the field was added.
  pub fn add_urlencoded_field(&mut self, row_ix: Option<usize>, cx: &mut Context<Self>) {
    self.record(cx, |this| {
      if let RestBody::FormUrlEncoded { fields } = &mut this.body {
        let ix = row_ix.map(|ix| min(ix + 1, fields.len())).unwrap_or(fields.len());
        let name = ki_utils::next_available_name("Param", fields.iter().map(|field| field.name.clone()));
        let field = FormField {
          name,
          value: Default::default(),
          disabled: false,
        };
        fields.insert(ix, field);
      }
    });
  }

  /// Starts a raw-content edit (e.g. when the raw body editor gets the focus): the changes made by
  /// [`RestData::set_body_raw_content`] until [`RestData::end_raw_content_edit`] are recorded as a single change in the
  /// history of the parent step, rather than one change per call.
  ///
  /// A raw-content edit still in progress is ended first.
  ///
  /// Must not be called while the parent step is being updated.
  pub fn begin_raw_content_edit(&mut self, cx: &mut Context<Self>) {
    self.end_raw_content_edit(cx);
    self.raw_edit_start = Some(self.state());
  }

  /// The request body.
  #[inline]
  pub fn body(&self) -> &RestBody {
    &self.body
  }

  pub fn delete_param(&mut self, row_ix: usize, cx: &mut Context<Self>) {
    if row_ix >= self.params.len() {
      error!("RestData::delete_param: Attempt to delete a non existing index: {row_ix}");
      return;
    }
    self.record(cx, |this| {
      this.params.remove(row_ix);
    });
  }

  /// Removes the field at `row_ix` from a URL-encoded body.
  ///
  /// Does nothing if the body is not URL-encoded, or if there is no field at `row_ix` (an error is logged).
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the field was removed.
  pub fn delete_urlencoded_field(&mut self, row_ix: usize, cx: &mut Context<Self>) {
    self.record(cx, |this| {
      if let RestBody::FormUrlEncoded { fields } = &mut this.body {
        if row_ix >= fields.len() {
          error!("RestData::delete_urlencoded_field: Attempt to delete a non existing index: {row_ix}");
          return;
        }
        fields.remove(row_ix);
      }
    });
  }

  pub fn duplicate_param(&mut self, row_ix: usize, cx: &mut Context<Self>) {
    let Some(mut param) = self.params.get(row_ix).cloned() else {
      error!("RestData::duplicate_param: Attempt to duplicate a non existing index: {row_ix}");
      return;
    };
    let name = ki_utils::next_available_name(&param.name, self.params.iter().map(|param| param.name.clone()));
    param.name = name;
    let ix = min(row_ix + 1, self.params.len());
    self.record(cx, |this| this.params.insert(ix, param));
  }

  /// Inserts a copy of the field at `row_ix` of a URL-encoded body right after it.
  ///
  /// The copy keeps the value and disabled flag of the original, and its name is suffixed (`_1`, `_2`...) so that it
  /// is unique among the fields. Does nothing if the body is not URL-encoded, or if there is no field at `row_ix`
  /// (an error is logged).
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the field was duplicated.
  pub fn duplicate_urlencoded_field(&mut self, row_ix: usize, cx: &mut Context<Self>) {
    self.record(cx, |this| {
      if let RestBody::FormUrlEncoded { fields } = &mut this.body {
        let Some(mut field) = fields.get(row_ix).cloned() else {
          error!("RestData::duplicate_urlencoded_field: Attempt to duplicate a non existing index: {row_ix}");
          return;
        };
        let name = ki_utils::next_available_name(&field.name, fields.iter().map(|field| field.name.clone()));
        field.name = name;
        let ix = min(row_ix + 1, fields.len());
        fields.insert(ix, field);
      }
    });
  }

  /// Returns a copy of this request, owned by the step `parent`.
  pub(crate) fn duplicate(&self, parent: WeakEntity<TestStep>) -> Self {
    Self {
      container: self.container.clone(),
      parent,
      method: self.method,
      endpoint: self.endpoint.clone(),
      params: self.params.clone(),
      body: self.body.clone(),
      raw_edit_start: None,
    }
  }

  /// Ends the raw-content edit in progress, if any, recording the change made since
  /// [`RestData::begin_raw_content_edit`] in the history of the parent step.
  ///
  /// Must not be called while the parent step is being updated.
  pub fn end_raw_content_edit(&mut self, cx: &mut Context<Self>) {
    if let Some(before) = self.raw_edit_start.take() {
      self.push_change(before, cx);
    }
  }

  #[inline]
  pub fn endpoint(&self) -> SharedString {
    self.endpoint.clone()
  }

  pub fn icon(&self) -> Icon {
    self.method.icon()
  }

  pub fn from_file(step: FileRestData, parent: WeakEntity<TestStep>, container: WeakEntity<TestsContainer>) -> Self {
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
      parent,
      method,
      endpoint: endpoint.into(),
      params,
      body,
      raw_edit_start: None,
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

  /// Records the change from `before` to the current state (if any) in the history of the parent step. Returns whether
  /// there was a change.
  fn push_change(&mut self, before: RestState, cx: &mut Context<Self>) -> bool {
    let after = self.state();
    if before == after {
      return false;
    }
    _ = self
      .parent
      .update(cx, |parent, _| parent.push_edit(Box::new(EditRest { before, after })));
    true
  }

  /// Applies `f` to this request, and records the change it made (if any) in the history of the parent step, so
  /// that it can be undone. A raw-content edit in progress is ended first, so that it is recorded before this change.
  ///
  /// Must not be called while the parent step is being updated: the change would not be recorded.
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when `f` changed the request.
  fn record(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Self)) -> bool {
    self.end_raw_content_edit(cx);
    let before = self.state();
    f(self);
    if !self.push_change(before, cx) {
      return false;
    }
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    cx.notify();
    true
  }

  /// Puts back a state of the request, without recording it.
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container, and [`RestDataEvent::DataChanged`].
  fn restore(&mut self, state: RestState, cx: &mut Context<Self>) {
    let RestState {
      method,
      endpoint,
      params,
      body,
    } = state;
    self.method = method;
    self.endpoint = endpoint;
    self.params = params;
    self.body = body;
    // A raw-content edit in progress now starts from the restored state
    if self.raw_edit_start.is_some() {
      self.raw_edit_start = Some(self.state());
    }
    _ = self.container.update(cx, |_, cx| cx.emit(TestsContainerEvent::TestsModified));
    cx.emit(StepDataEvent::DataChanged);
    cx.notify();
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
    self.record(cx, |this| {
      this.body = body;
      this.update_generated_content_type_header();
    });
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
  /// The change is recorded right away, unless a raw-content edit is in progress (see
  /// [`RestData::begin_raw_content_edit`]): it is then recorded along with the others when the edit ends.
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the text changed.
  pub fn set_body_raw_content(&mut self, updated_content: SharedString, cx: &mut Context<Self>) {
    if self.raw_edit_start.is_none() {
      self.record(cx, |this| {
        if let RestBody::Raw { content, .. } = &mut this.body {
          *content = updated_content;
        }
      });
    } else if let RestBody::Raw { content, .. } = &mut self.body
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
    self.record(cx, |this| {
      if let RestBody::Raw { content_type, .. } = &mut this.body
        && *content_type != updated_content_type
      {
        *content_type = updated_content_type;
        this.update_generated_content_type_header();
      }
    });
  }

  pub fn set_endpoint(&mut self, endpoint: SharedString, cx: &mut Context<Self>) {
    self.record(cx, |this| this.endpoint = endpoint);
  }

  pub fn set_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) -> bool {
    self.record(cx, |this| this.method = method)
  }

  /// Returns a copy of the editable state of the request.
  fn state(&self) -> RestState {
    RestState {
      method: self.method,
      endpoint: self.endpoint.clone(),
      params: self.params.clone(),
      body: self.body.clone(),
    }
  }

  pub fn update_params(&mut self, row_ix: usize, param: RestParam, cx: &mut Context<Self>) {
    if row_ix >= self.params.len() {
      error!(
        "RestData::update_params: Attempted to update a non-existent index: {}",
        row_ix
      );
      return;
    }
    self.record(cx, |this| this.params[row_ix] = param);
  }

  /// Replaces the field at `row_ix` of a URL-encoded body with `field`.
  ///
  /// Does nothing if the body is not URL-encoded, if `field` is equal to the current one, or if there is no field at
  /// `row_ix` (an error is logged).
  ///
  /// # Events
  /// Emits [`TestsContainerEvent::TestsModified`] on the owning container when the field changed.
  pub fn update_urlencoded_field(&mut self, row_ix: usize, field: FormField, cx: &mut Context<Self>) {
    self.record(cx, |this| {
      if let RestBody::FormUrlEncoded { fields } = &mut this.body {
        if row_ix >= fields.len() {
          error!(
            "RestData::update_urlencoded_field: Attempted to update a non-existent index: {}",
            row_ix
          );
          return;
        }
        fields[row_ix] = field;
      }
    });
  }
}

impl EventEmitter<StepDataEvent> for RestData {}

/// The editable state of a [`RestData`], as saved by its undo edits.
#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) struct RestState {
  method: HttpMethod,
  endpoint: SharedString,
  params: Vec<RestParam>,
  body: RestBody,
}

/// A change of a [`RestData`], from the `before` state to the `after` one.
///
/// Whole states are saved rather than the operation that was made, since a single operation may change several parts
/// of the request (e.g. a new body changes the generated `Content-Type` header).
struct EditRest {
  before: RestState,
  after: RestState,
}

impl EditRest {
  fn apply(target: &TestStep, state: &RestState, cx: &mut Context<TestStep>) {
    if let StepData::Rest(rest) = &target.data {
      rest.update(cx, |rest, cx| rest.restore(state.clone(), cx));
    }
  }
}

impl Edit<TestStep> for EditRest {
  fn undo(&self, target: &mut TestStep, cx: &mut Context<TestStep>) {
    Self::apply(target, &self.before, cx);
  }

  fn redo(&self, target: &mut TestStep, cx: &mut Context<TestStep>) {
    Self::apply(target, &self.after, cx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::test_fixtures::{load, record_events};
  use gpui_kit::{AppContext, Entity, TestAppContext};
  use ki_utils::TestPath;
  use ki_utils::http::{
    CONTENT_TYPE_FORM_DATA, CONTENT_TYPE_FORM_URL_ENCODED, CONTENT_TYPE_JSON, CONTENT_TYPE_PLAIN_TEXT, CONTENT_TYPE_XML,
  };
  use ki_utils::id::KId;
  use ki_utils::undo::UndoEntity;

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
      let rest = RestData::from_file(file, WeakEntity::new_invalid(), tests.downgrade());

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
    let rest = RestData::from_file(
      file_rest(vec![], FileRestBody::None),
      WeakEntity::new_invalid(),
      tests.downgrade(),
    );
    assert!(rest.params().is_empty(), "unexpected params {:?}", rest.params());
  }

  #[gpui_kit::test]
  fn form_bodies_generate_their_content_type_header(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    for (body, mime) in [
      (FileRestBody::FormUrlEncoded { fields: vec![] }, CONTENT_TYPE_FORM_URL_ENCODED),
      (FileRestBody::Multipart { parts: vec![] }, CONTENT_TYPE_FORM_DATA),
    ] {
      let rest = RestData::from_file(file_rest(vec![], body), WeakEntity::new_invalid(), tests.downgrade());
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
    let rest = RestData::from_file(file.clone(), WeakEntity::new_invalid(), tests.downgrade());

    assert_eq!(rest.params().iter().filter(|param| param.generated()).count(), 1);
    assert_eq!(rest.get_file(), file);
  }

  /// Loads `file` as a live REST step owned by `tests`.
  fn rest_entity(file: FileRestData, tests: &Entity<TestsContainer>, cx: &mut TestAppContext) -> Entity<RestData> {
    let container = tests.downgrade();
    cx.new(|_| RestData::from_file(file, WeakEntity::new_invalid(), container))
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
    assert_eq!(RestBody::ALL, [REST_BODY_NONE, REST_BODY_RAW, REST_BODY_FORM_URL_ENCODED]);
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

  fn field(name: &str, value: &str, disabled: bool) -> FormField {
    FormField {
      name: name.into(),
      value: value.into(),
      disabled,
    }
  }

  /// Loads a REST step whose URL-encoded body holds `fields`.
  fn urlencoded_entity(fields: &[FormField], tests: &Entity<TestsContainer>, cx: &mut TestAppContext) -> Entity<RestData> {
    let fields = fields.iter().map(FileFormField::from).collect();
    rest_entity(file_rest(vec![], FileRestBody::FormUrlEncoded { fields }), tests, cx)
  }

  /// Fields of the URL-encoded body of `rest`.
  fn fields(rest: &Entity<RestData>, cx: &TestAppContext) -> Vec<FormField> {
    rest.read_with(cx, |rest, _| match rest.body() {
      RestBody::FormUrlEncoded { fields } => fields.clone(),
      body => panic!("expected a URL-encoded body, got {body:?}"),
    })
  }

  #[gpui_kit::test]
  fn add_urlencoded_field_inserts_after_the_given_row(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false), field("b", "2", false)], &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.add_urlencoded_field(Some(0), cx));
    assert_eq!(
      fields(&rest, cx),
      vec![field("a", "1", false), field("Param", "", false), field("b", "2", false)]
    );
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn add_urlencoded_field_appends_without_row_or_past_the_end(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false)], &tests, cx);

    rest.update(cx, |rest, cx| rest.add_urlencoded_field(None, cx));
    rest.update(cx, |rest, cx| rest.add_urlencoded_field(Some(42), cx));
    assert_eq!(
      fields(&rest, cx),
      vec![field("a", "1", false), field("Param", "", false), field("Param_1", "", false)]
    );
  }

  #[gpui_kit::test]
  fn add_urlencoded_field_names_are_unique_among_fields_only(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    // A param named "Param" must not affect the name of the new field
    let file = file_rest(
      vec![file_param(RestParamType::Query, "Param", "1")],
      FileRestBody::FormUrlEncoded { fields: vec![] },
    );
    let rest = rest_entity(file, &tests, cx);

    rest.update(cx, |rest, cx| rest.add_urlencoded_field(None, cx));
    assert_eq!(fields(&rest, cx), vec![field("Param", "", false)]);
  }

  #[gpui_kit::test]
  fn delete_urlencoded_field_removes_the_row(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false), field("b", "2", false)], &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.delete_urlencoded_field(0, cx));
    assert_eq!(fields(&rest, cx), vec![field("b", "2", false)]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn delete_urlencoded_field_ignores_a_missing_row(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false)], &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.delete_urlencoded_field(1, cx));
    assert_eq!(fields(&rest, cx), vec![field("a", "1", false)]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn duplicate_urlencoded_field_inserts_a_renamed_copy_after_the_row(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", true), field("b", "2", false)], &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.duplicate_urlencoded_field(0, cx));
    rest.update(cx, |rest, cx| rest.duplicate_urlencoded_field(0, cx));
    assert_eq!(
      fields(&rest, cx),
      vec![
        field("a", "1", true),
        field("a_2", "1", true),
        field("a_1", "1", true),
        field("b", "2", false),
      ]
    );
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 2]);
  }

  #[gpui_kit::test]
  fn duplicate_urlencoded_field_ignores_a_missing_row(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false)], &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.duplicate_urlencoded_field(1, cx));
    assert_eq!(fields(&rest, cx), vec![field("a", "1", false)]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn update_urlencoded_field_notifies_changes_only(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false), field("b", "2", false)], &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.update_urlencoded_field(1, field("b", "2", false), cx));
    assert!(events.borrow().is_empty());

    rest.update(cx, |rest, cx| rest.update_urlencoded_field(1, field("c", "3", true), cx));
    assert_eq!(fields(&rest, cx), vec![field("a", "1", false), field("c", "3", true)]);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified]);
  }

  #[gpui_kit::test]
  fn update_urlencoded_field_ignores_a_missing_row(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false)], &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| rest.update_urlencoded_field(1, field("b", "2", false), cx));
    assert_eq!(fields(&rest, cx), vec![field("a", "1", false)]);
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn urlencoded_field_methods_ignore_other_bodies(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = rest_entity(file_rest(vec![], raw_body(ContentType::Json)), &tests, cx);
    let events = record_events(&tests, cx);

    rest.update(cx, |rest, cx| {
      rest.add_urlencoded_field(None, cx);
      rest.duplicate_urlencoded_field(0, cx);
      rest.update_urlencoded_field(0, field("a", "1", false), cx);
      rest.delete_urlencoded_field(0, cx);
    });
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.body().clone()),
      raw(ContentType::Json, "{}")
    );
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn urlencoded_field_edits_are_saved(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let rest = urlencoded_entity(&[field("a", "1", false)], &tests, cx);

    rest.update(cx, |rest, cx| {
      rest.add_urlencoded_field(Some(0), cx);
      rest.update_urlencoded_field(1, field("b", "2", true), cx);
    });
    let expected = [field("a", "1", false), field("b", "2", true)];
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.get_file().body),
      FileRestBody::FormUrlEncoded {
        fields: expected.iter().map(FileFormField::from).collect()
      }
    );
  }

  /// Creates a new GET Rest step owned by `tests`, and returns it along with its request.
  fn rest_step(tests: &Entity<TestsContainer>, cx: &mut TestAppContext) -> (Entity<TestStep>, Entity<RestData>) {
    let parent = TestPath::Case(KId::new(), KId::new());
    let step = cx.new(|cx| TestStep::new_rest(&parent, "Rest".into(), HttpMethod::Get, tests.downgrade(), cx));
    let StepData::Rest(rest) = step.read_with(cx, |step, _| step.data().clone()) else {
      panic!("should be a rest step");
    };
    (step, rest)
  }

  fn can_undo(step: &Entity<TestStep>, cx: &mut TestAppContext) -> bool {
    step.update(cx, |step, _| step.history().can_undo())
  }

  #[gpui_kit::test]
  fn edits_are_undone_and_redone_in_order(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = rest_step(&tests, cx);
    let mut states = vec![rest.read_with(cx, |rest, _| rest.state())];
    type RestEdit = Box<dyn Fn(&mut RestData, &mut Context<RestData>)>;
    let edits: Vec<RestEdit> = vec![
      Box::new(|rest, cx| _ = rest.set_method(HttpMethod::Post, cx)),
      Box::new(|rest, cx| rest.set_endpoint("/items".into(), cx)),
      Box::new(|rest, cx| rest.add_param(None, cx)),
      Box::new(|rest, cx| rest.set_body(RestBody::from(REST_BODY_RAW), cx)),
      Box::new(|rest, cx| rest.set_body_raw_content("{}".into(), cx)),
      Box::new(|rest, cx| rest.set_body_raw_content_type(ContentType::Json, cx)),
      Box::new(|rest, cx| rest.duplicate_param(0, cx)),
      Box::new(|rest, cx| rest.delete_param(0, cx)),
    ];
    for edit in &edits {
      rest.update(cx, |rest, cx| edit(rest, cx));
      let state = rest.read_with(cx, |rest, _| rest.state());
      assert_ne!(states.last(), Some(&state), "every edit should change the request");
      states.push(state);
    }

    for expected in states.iter().rev().skip(1) {
      step.update(cx, |step, cx| step.undo(cx));
      assert_eq!(&rest.read_with(cx, |rest, _| rest.state()), expected);
    }
    assert!(!can_undo(&step, cx));
    for expected in states.iter().skip(1) {
      step.update(cx, |step, cx| step.redo(cx));
      assert_eq!(&rest.read_with(cx, |rest, _| rest.state()), expected);
    }
  }

  #[gpui_kit::test]
  fn urlencoded_field_edits_are_undone(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = rest_step(&tests, cx);
    rest.update(cx, |rest, cx| rest.set_body(RestBody::from(REST_BODY_FORM_URL_ENCODED), cx));
    let empty_form = rest.read_with(cx, |rest, _| rest.state());
    rest.update(cx, |rest, cx| {
      rest.add_urlencoded_field(None, cx);
      rest.duplicate_urlencoded_field(0, cx);
      rest.update_urlencoded_field(
        1,
        FormField {
          name: "name".into(),
          value: "value".into(),
          disabled: true,
        },
        cx,
      );
      rest.delete_urlencoded_field(0, cx);
    });
    for _ in 0..4 {
      step.update(cx, |step, cx| step.undo(cx));
    }
    assert_eq!(rest.read_with(cx, |rest, _| rest.state()), empty_form);
  }

  #[gpui_kit::test]
  fn undoing_a_body_change_restores_the_generated_header(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = rest_step(&tests, cx);
    rest.update(cx, |rest, cx| rest.set_body(RestBody::from(REST_BODY_FORM_URL_ENCODED), cx));
    assert_eq!(
      generated_headers(&rest, cx),
      vec![SharedString::from(CONTENT_TYPE_FORM_URL_ENCODED)]
    );
    step.update(cx, |step, cx| step.undo(cx));
    assert!(generated_headers(&rest, cx).is_empty());
    assert_eq!(rest.read_with(cx, |rest, _| rest.body().clone()), RestBody::None);
  }

  #[gpui_kit::test]
  fn unchanged_edits_are_not_recorded(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let events = record_events(&tests, cx);
    let (step, rest) = rest_step(&tests, cx);
    rest.update(cx, |rest, cx| {
      assert!(!rest.set_method(HttpMethod::Get, cx));
      rest.set_endpoint(rest.endpoint(), cx);
      rest.set_body(RestBody::None, cx);
      rest.set_body_raw_content("ignored, the body is not raw".into(), cx);
      rest.add_urlencoded_field(None, cx);
      rest.delete_param(3, cx);
      rest.duplicate_param(3, cx);
    });
    assert!(!can_undo(&step, cx));
    assert!(events.borrow().is_empty());
  }

  #[gpui_kit::test]
  fn undo_and_redo_notify_the_editors_and_the_container(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = rest_step(&tests, cx);
    rest.update(cx, |rest, cx| rest.set_endpoint("/items".into(), cx));
    let events = record_events(&tests, cx);
    let changes = std::rc::Rc::new(std::cell::Cell::new(0));
    let _sub = cx.update(|cx| {
      let changes = changes.clone();
      cx.subscribe(&rest, move |_, _: &StepDataEvent, _| changes.set(changes.get() + 1))
    });
    step.update(cx, |step, cx| step.undo(cx));
    step.update(cx, |step, cx| step.redo(cx));
    assert_eq!(changes.get(), 2);
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 2]);
  }

  /// Creates a Rest step with an empty raw body, with no change to undo.
  fn raw_rest_step(tests: &Entity<TestsContainer>, cx: &mut TestAppContext) -> (Entity<TestStep>, Entity<RestData>) {
    let (step, rest) = rest_step(tests, cx);
    rest.update(cx, |rest, cx| rest.set_body(RestBody::from(REST_BODY_RAW), cx));
    // Drop the body type change, so that only the raw-content edits are in the history
    step.update(cx, |step, _| *step.history() = Default::default());
    (step, rest)
  }

  fn raw_content(rest: &Entity<RestData>, cx: &mut TestAppContext) -> SharedString {
    rest.read_with(cx, |rest, _| match rest.body() {
      RestBody::Raw { content, .. } => content.clone(),
      body => panic!("expected a raw body, got {body:?}"),
    })
  }

  #[gpui_kit::test]
  fn raw_content_edit_is_recorded_as_one_change(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = raw_rest_step(&tests, cx);
    let events = record_events(&tests, cx);
    rest.update(cx, |rest, cx| {
      rest.begin_raw_content_edit(cx);
      for content in ["{", "{}", "{ }"] {
        rest.set_body_raw_content(content.into(), cx);
      }
    });
    // Each change is saved right away, but not recorded until the edit ends
    assert_eq!(*events.borrow(), vec![TestsContainerEvent::TestsModified; 3]);
    assert!(!can_undo(&step, cx));
    rest.update(cx, |rest, cx| rest.end_raw_content_edit(cx));
    assert_eq!(events.borrow().len(), 3);

    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(raw_content(&rest, cx), "");
    assert!(!can_undo(&step, cx));
    step.update(cx, |step, cx| step.redo(cx));
    assert_eq!(raw_content(&rest, cx), "{ }");
  }

  #[gpui_kit::test]
  fn unchanged_raw_content_edit_is_not_recorded(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = raw_rest_step(&tests, cx);
    rest.update(cx, |rest, cx| {
      rest.begin_raw_content_edit(cx);
      rest.set_body_raw_content("x".into(), cx);
      rest.set_body_raw_content("".into(), cx);
      rest.end_raw_content_edit(cx);
      // Ending again does nothing
      rest.end_raw_content_edit(cx);
    });
    assert!(!can_undo(&step, cx));
  }

  #[gpui_kit::test]
  fn other_edits_end_the_raw_content_edit_first(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = raw_rest_step(&tests, cx);
    rest.update(cx, |rest, cx| {
      rest.begin_raw_content_edit(cx);
      rest.set_body_raw_content("{}".into(), cx);
      rest.set_endpoint("/items".into(), cx);
      // The edit is over: this change is recorded on its own
      rest.set_body_raw_content("{ }".into(), cx);
    });

    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(raw_content(&rest, cx), "{}");
    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.endpoint()),
      ki_utils::http::LOCALHOST_ENDPOINT.clone()
    );
    assert_eq!(raw_content(&rest, cx), "{}");
    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(raw_content(&rest, cx), "");
    assert!(!can_undo(&step, cx));
  }

  #[gpui_kit::test]
  fn a_new_raw_content_edit_ends_the_previous_one(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = raw_rest_step(&tests, cx);
    rest.update(cx, |rest, cx| {
      rest.begin_raw_content_edit(cx);
      rest.set_body_raw_content("a".into(), cx);
      rest.begin_raw_content_edit(cx);
      rest.set_body_raw_content("ab".into(), cx);
      rest.end_raw_content_edit(cx);
    });
    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(raw_content(&rest, cx), "a");
    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(raw_content(&rest, cx), "");
  }

  #[gpui_kit::test]
  fn undo_during_a_raw_content_edit_restarts_it(cx: &mut TestAppContext) {
    let tests = load(Default::default(), Default::default(), cx);
    let (step, rest) = raw_rest_step(&tests, cx);
    rest.update(cx, |rest, cx| rest.set_endpoint("/items".into(), cx));
    rest.update(cx, |rest, cx| {
      rest.begin_raw_content_edit(cx);
      rest.set_body_raw_content("{}".into(), cx);
    });
    // The pending typing is not in the history yet: undo reverts the endpoint, from the current state
    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(
      rest.read_with(cx, |rest, _| rest.endpoint()),
      ki_utils::http::LOCALHOST_ENDPOINT.clone()
    );
    rest.update(cx, |rest, cx| {
      rest.set_body_raw_content("{ }".into(), cx);
      rest.end_raw_content_edit(cx);
    });
    // Only the typing made after the undo is recorded, from the restored state
    step.update(cx, |step, cx| step.undo(cx));
    assert_eq!(raw_content(&rest, cx), "");
    assert!(!can_undo(&step, cx));
  }
}
