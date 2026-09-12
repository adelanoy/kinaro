use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
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
  pub overrides: HashMap<Uuid, String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
pub enum FileVariableKind {
  Text,
  PasswordClear,
  PasswordEncrypt,
}

#[derive(Serialize, Deserialize, Eq, Clone, Debug)]
pub struct FileProfile {
  pub id: Uuid,
  pub name: String,
  pub description: String,
}

impl PartialEq for FileProfile {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl PartialOrd for FileProfile {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for FileProfile {
  fn cmp(&self, other: &Self) -> Ordering {
    self.name.cmp(&other.name)
  }
}
