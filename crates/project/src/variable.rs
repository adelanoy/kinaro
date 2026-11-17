use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FileProjectVariables {
  pub variables: Vec<FileVariable>,
  pub profiles: Vec<FileProfile>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct FileVariable {
  pub id: Uuid,
  pub name: String,
  pub description: String,
  pub kind: FileVariableKind,
  pub value: String,
  pub overrides: BTreeMap<Uuid, String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
pub enum FileVariableKind {
  Text,
  PasswordClear,
  PasswordEncrypt,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct FileProfile {
  pub id: Uuid,
  pub name: String,
  pub description: String,
}
