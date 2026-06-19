use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Variables {
    pub variables: Vec<Variable>,
    pub profiles: BTreeSet<Profile>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct Variable {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub kind: VariableKind,
    pub value: String,
    pub overrides: HashMap<Uuid, String>
}

impl Default for Variable {
    fn default() -> Self {
        Variable {
            id: Uuid::new_v4(),
            name: String::new(),
            description: String::new(),
            kind: VariableKind::Text,
            value: String::new(),
            overrides: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub enum VariableKind {
    Text,
    PasswordClear,
    PasswordEncrypt,
}

#[derive(Serialize, Deserialize, Eq, Ord, Clone, Debug)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub description: String,
}

impl PartialEq for Profile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Profile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for Profile {

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}
