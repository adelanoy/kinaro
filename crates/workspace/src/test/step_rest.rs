use gpui_kit::SharedString;
use ki_project::{
  FileFormField, FileMultipartPart, FileRestBody, FileRestMethod, FileRestParam, FileRestParamType, FileRestStep,
};
use std::path::PathBuf;

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
pub struct RestStep {
  pub method: RestMethod,
  pub endpoint: SharedString,
  pub params: Vec<RestParam>,
  pub body: RestBody,
}

impl From<FileRestStep> for RestStep {
  fn from(step: FileRestStep) -> Self {
    let FileRestStep {
      method,
      endpoint,
      params,
      body,
    } = step;
    RestStep {
      method: method.into(),
      endpoint: endpoint.into(),
      params: params.into_iter().map(Into::into).collect(),
      body: body.into(),
    }
  }
}

impl From<&RestStep> for FileRestStep {
  fn from(step: &RestStep) -> Self {
    let RestStep {
      method,
      endpoint,
      params,
      body,
    } = step;
    FileRestStep {
      method: method.into(),
      endpoint: endpoint.to_string(),
      params: params.iter().map(Into::into).collect(),
      body: body.into(),
    }
  }
}
