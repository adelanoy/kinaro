use project_file::{Profile, Variable, VariableKind, Variables};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct WorkspaceVariables {
    pub variables: Vec<WorkspaceVariable>,
    pub profiles: Vec<WorkspaceProfile>,
}

impl WorkspaceVariables {
    pub(crate) fn from_file(file_vars: &Variables) -> Self {
        let profiles = file_vars
            .profiles
            .iter()
            .map(|p| WorkspaceProfile::from_file(p))
            .collect::<Vec<WorkspaceProfile>>();

        let variables = file_vars
            .variables
            .iter()
            .map(|file_var| {
                let overrides = profiles
                    .iter()
                    .map(|profile| (profile.id, file_var.overrides.get(&profile.id).cloned()))
                    .collect::<HashMap<Uuid, Option<String>>>();
                WorkspaceVariable {
                    id: file_var.id,
                    name: file_var.name.clone(),
                    description: file_var.description.clone(),
                    kind: WorkspaceVariableKind::from(&file_var.kind),
                    reference_value: file_var.value.clone(),
                    overrides,
                }
            })
            .collect();

        WorkspaceVariables {
            variables,
            profiles,
        }
    }

    pub(crate) fn get_file(&self) -> Variables {
        let profiles = self
            .profiles
            .iter()
            .map(|p| p.get_file())
            .collect::<BTreeSet<Profile>>();

        let mut variables = vec![];
        for workspace_var in &self.variables {
            let overrides = workspace_var
                .overrides
                .iter()
                .filter(|(_, value)| value.is_some())
                .map(|(id, value)| (*id, value.clone().unwrap()))
                .collect::<HashMap<Uuid, String>>();
            variables.push(Variable {
                id: workspace_var.id,
                name: workspace_var.name.clone(),
                description: workspace_var.description.clone(),
                kind: VariableKind::from(&workspace_var.kind),
                value: workspace_var.reference_value.clone(),
                overrides,
            });
        }

        Variables {
            variables,
            profiles,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct WorkspaceVariable {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub kind: WorkspaceVariableKind,
    pub reference_value: String,
    pub overrides: HashMap<Uuid, Option<String>>,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum WorkspaceVariableKind {
    Text,
    PasswordClear,
    PasswordEncrypt,
}

impl From<&VariableKind> for WorkspaceVariableKind {
    fn from(value: &VariableKind) -> Self {
        match value {
            VariableKind::Text => WorkspaceVariableKind::Text,
            VariableKind::PasswordClear => WorkspaceVariableKind::PasswordClear,
            VariableKind::PasswordEncrypt => WorkspaceVariableKind::PasswordEncrypt,
        }
    }
}

impl From<&WorkspaceVariableKind> for VariableKind {
    fn from(value: &WorkspaceVariableKind) -> Self {
        match value {
            WorkspaceVariableKind::Text => VariableKind::Text,
            WorkspaceVariableKind::PasswordClear => VariableKind::PasswordClear,
            WorkspaceVariableKind::PasswordEncrypt => VariableKind::PasswordEncrypt,
        }
    }
}

#[derive(Eq, Ord, Clone, Debug)]
pub struct WorkspaceProfile {
    pub id: Uuid,
    pub name: String,
    pub description: String,
}

impl WorkspaceProfile {
    fn from_file(file_profile: &Profile) -> Self {
        Self {
            id: file_profile.id,
            name: file_profile.name.clone(),
            description: file_profile.description.clone(),
        }
    }

    fn get_file(&self) -> Profile {
        Profile {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
        }
    }
}

impl PartialEq for WorkspaceProfile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id || self.name == other.name
    }
}

impl PartialOrd for WorkspaceProfile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}