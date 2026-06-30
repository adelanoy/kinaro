use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FileProjectVariables {
    pub variables: Vec<FileVariable>,
    pub profiles: BTreeSet<FileProfile>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct FileVariable {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub kind: VariableKind,
    pub value: String,
    pub overrides: HashMap<Uuid, String>
}

impl Default for FileVariable {
    fn default() -> Self {
        FileVariable {
            id: Uuid::new_v4(),
            name: String::new(),
            description: String::new(),
            kind: VariableKind::Text,
            value: String::new(),
            overrides: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
pub enum VariableKind {
    Text,
    PasswordClear,
    PasswordEncrypt,
}

#[derive(Serialize, Deserialize, Eq, Ord, Clone, Debug)]
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

impl Hash for FileProfile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for FileProfile {

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}
