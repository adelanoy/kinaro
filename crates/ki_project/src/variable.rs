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
    pub kind: VariableKind,
    pub value: String,
}

impl Default for FileVariable {
    fn default() -> Self {
        FileVariable {
            id: Uuid::new_v4(),
            name: String::new(),
            description: String::new(),
            kind: VariableKind::Text,
            value: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
pub enum VariableKind {
    Text,
    PasswordClear,
    PasswordEncrypt,
}

#[derive(Serialize, Deserialize, Eq, Clone, Debug)]
pub struct FileProfile {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub overrides: HashMap<Uuid, String>,
}

impl PartialEq for FileProfile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for FileProfile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for FileProfile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
