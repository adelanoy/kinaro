use gpui::SharedString;
use gpui_component::select::SelectItem;
use ki_project::{Profile, Variable, VariableKind, Variables};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct WorkspaceVariables {
    pub variables: Vec<WorkspaceVariable>,
    pub profiles: Vec<WorkspaceProfile>,
}

impl WorkspaceVariables {
    /// Merge the current profiles with the new ones
    /// 
    /// Returns true if any merge occurred and project should notify listeners
    pub(crate) fn update_profiles(&mut self, profiles: Vec<WorkspaceProfile>) -> bool {
        let mut must_notify = false;
        for profile in profiles {
            let pos_existing = self.profiles.iter().position(|p| p.id == profile.id);
            match pos_existing {
                None => {
                    self.profiles.push(profile);
                    must_notify = true;
                }
                Some(pos) => {
                    let current_profile = self.profiles.get_mut(pos).unwrap();
                    if current_profile != &profile {
                        current_profile.name = profile.name;
                        current_profile.description = profile.description;
                        must_notify = true;
                    }
                }
            }
        }
        must_notify
    }

    /// Merge the current variables with the new ones
    /// 
    /// Returns true if any merge occurred and project should notify listeners
    pub(crate) fn update_variables(&mut self, variables: Vec<WorkspaceVariable>) -> bool{
        let mut must_notify = false;
        for variable in variables {
            let pos_existing = self.profiles.iter().position(|p| p.id == variable.id);
            match pos_existing {
                None => {
                    self.variables.push(variable);
                    must_notify = true;
                }
                Some(pos) => {
                    let current_profile = self.variables.get_mut(pos).unwrap();
                    if current_profile != &variable {
                        current_profile.name = variable.name;
                        current_profile.description = variable.description;
                        must_notify = true;
                    }
                }
            }
        }
        must_notify
    }

    pub fn from_file(file_vars: &Variables) -> Self {
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
                    name: SharedString::new(&file_var.name),
                    description: SharedString::new(&file_var.description),
                    kind: WorkspaceVariableKind::from(&file_var.kind),
                    reference_value: SharedString::new(&file_var.value),
                    overrides,
                }
            })
            .collect::<Vec<WorkspaceVariable>>();

        WorkspaceVariables {
            variables,
            profiles,
        }
    }

    pub fn to_file(&self) -> Variables {
        let profiles = self
            .profiles
            .iter()
            .map(|p| p.get_file())
            .collect::<BTreeSet<Profile>>();

        let mut variables = vec![];
        for workspace_var in &self.variables {
            let workspace_var = workspace_var;
            let overrides = workspace_var
                .overrides
                .iter()
                .filter(|(_, value)| value.is_some())
                .map(|(id, value)| (*id, value.clone().unwrap()))
                .collect::<HashMap<Uuid, String>>();
            variables.push(Variable {
                id: workspace_var.id,
                name: workspace_var.name.to_string(),
                description: workspace_var.description.to_string(),
                kind: VariableKind::from(&workspace_var.kind),
                value: workspace_var.reference_value.to_string(),
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
    pub name: SharedString,
    pub description: SharedString,
    pub kind: WorkspaceVariableKind,
    pub reference_value: SharedString,
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

#[derive(Eq, PartialEq, Ord, Clone, Debug)]
pub struct WorkspaceProfile {
    pub id: Uuid,
    pub name: SharedString,
    pub description: SharedString,
}

impl WorkspaceProfile {
    fn from_file(file_profile: &Profile) -> Self {
        Self {
            id: file_profile.id,
            name: SharedString::new(&file_profile.name),
            description: SharedString::new(&file_profile.description),
        }
    }

    fn get_file(&self) -> Profile {
        Profile {
            id: self.id,
            name: self.name.to_string(),
            description: self.description.to_string(),
        }
    }
}

impl PartialOrd for WorkspaceProfile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl SelectItem for WorkspaceProfile {
    type Value = Uuid;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}
