use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum FileRestMethod {
  Get,
  Post,
  Put,
  Delete,
  Patch,
  Head,
  Options,
}

/// Where a [`FileRestParam`] is placed in the outgoing request.
#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FileRestParamType {
  Path,
  Query,
  Header,
  Cookie,
}

/// A name/value pair sent as part of the request.
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct FileRestParam {
  #[serde(rename = "type")]
  pub param_type: FileRestParamType,
  pub name: String,
  pub value: String,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disabled: bool,
}

/// A field of a `application/x-www-form-urlencoded` body.
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct FileFormField {
  pub name: String,
  pub value: String,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disabled: bool,
}

/// A part of a `multipart/form-data` body.
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(tag = "kind")]
pub enum FileMultipartPart {
  Text {
    name: String,
    value: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    disabled: bool,
  },
  File {
    name: String,
    path: PathBuf,
    /// Content type of the part, guessed from the file when absent.
    #[serde(default, rename = "contentType", skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    disabled: bool,
  },
}

/// The request body. Its variant determines the `Content-Type` sent with the request.
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug, Default)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FileRestBody {
  #[default]
  None,
  /// Free text (JSON, XML, plain text...) sent as-is.
  Raw {
    #[serde(rename = "contentType")]
    content_type: String,
    /// One entry per line, so version control diffs only the lines that
    /// actually changed instead of one escaped multiline string.
    lines: Vec<String>,
  },
  FormUrlEncoded {
    fields: Vec<FileFormField>,
  },
  Multipart {
    parts: Vec<FileMultipartPart>,
  },
  /// The raw bytes of a single file.
  Binary {
    path: PathBuf,
  },
}

impl FileRestBody {
  pub fn is_none(&self) -> bool {
    matches!(self, FileRestBody::None)
  }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileRestStep {
  pub method: FileRestMethod,
  pub endpoint: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub params: Vec<FileRestParam>,
  #[serde(default, skip_serializing_if = "FileRestBody::is_none")]
  pub body: FileRestBody,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn param(param_type: FileRestParamType, name: &str, value: &str) -> FileRestParam {
    FileRestParam {
      param_type,
      name: name.into(),
      value: value.into(),
      disabled: false,
    }
  }

  fn roundtrip(step: &FileRestStep) -> FileRestStep {
    serde_json::from_value(serde_json::to_value(step).unwrap()).unwrap()
  }

  #[test]
  fn minimal_step_omits_params_and_body() {
    let step = FileRestStep {
      method: FileRestMethod::Get,
      endpoint: "/users".into(),
      params: vec![],
      body: FileRestBody::None,
    };

    let value = serde_json::to_value(&step).unwrap();

    assert_eq!(value, json!({ "method": "GET", "endpoint": "/users" }));
    assert_eq!(serde_json::from_value::<FileRestStep>(value).unwrap(), step);
  }

  #[test]
  fn params_serialize_with_type_and_skip_disabled_when_false() {
    let mut disabled = param(FileRestParamType::Header, "X-Trace", "1");
    disabled.disabled = true;
    let step = FileRestStep {
      method: FileRestMethod::Get,
      endpoint: "/users/{id}".into(),
      params: vec![param(FileRestParamType::Path, "id", "42"), disabled],
      body: FileRestBody::None,
    };

    let value = serde_json::to_value(&step).unwrap();

    assert_eq!(
      value["params"],
      json!([
        { "type": "path", "name": "id", "value": "42" },
        { "type": "header", "name": "X-Trace", "value": "1", "disabled": true },
      ])
    );
    assert_eq!(roundtrip(&step), step);
  }

  #[test]
  fn raw_body_is_tagged_and_stored_by_line() {
    let step = FileRestStep {
      method: FileRestMethod::Post,
      endpoint: "/users".into(),
      params: vec![],
      body: FileRestBody::Raw {
        content_type: "application/json".into(),
        lines: vec!["{".into(), "}".into()],
      },
    };

    let value = serde_json::to_value(&step).unwrap();

    assert_eq!(
      value["body"],
      json!({ "type": "raw", "contentType": "application/json", "lines": ["{", "}"] })
    );
    assert_eq!(roundtrip(&step), step);
  }

  #[test]
  fn form_and_binary_bodies_roundtrip() {
    for body in [
      FileRestBody::FormUrlEncoded {
        fields: vec![FileFormField {
          name: "a".into(),
          value: "1".into(),
          disabled: false,
        }],
      },
      FileRestBody::Multipart {
        parts: vec![
          FileMultipartPart::Text {
            name: "title".into(),
            value: "doc".into(),
            disabled: false,
          },
          FileMultipartPart::File {
            name: "file".into(),
            path: "data/doc.pdf".into(),
            content_type: Some("application/pdf".into()),
            disabled: true,
          },
        ],
      },
      FileRestBody::Binary {
        path: "data/blob.bin".into(),
      },
    ] {
      let step = FileRestStep {
        method: FileRestMethod::Put,
        endpoint: "/upload".into(),
        params: vec![],
        body,
      };
      assert_eq!(roundtrip(&step), step);
    }
  }

  #[test]
  fn multipart_part_is_tagged_by_kind() {
    let part = FileMultipartPart::File {
      name: "f".into(),
      path: "a.txt".into(),
      content_type: None,
      disabled: false,
    };

    assert_eq!(
      serde_json::to_value(&part).unwrap(),
      json!({ "kind": "File", "name": "f", "path": "a.txt" })
    );
  }
}
