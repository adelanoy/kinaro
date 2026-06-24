use crate::workspace::error::ProjectError;
use crate::workspace::project;
use gpui::SharedString;
use gpui_component::select::SelectItem;
use ki_project::{Profile, Variable, VariableKind, Variables};
use log::warn;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct WorkspaceVariables {
    pub variables: Vec<WorkspaceVariable>,
    pub profiles: Vec<WorkspaceProfile>,
}

impl WorkspaceVariables {}

impl WorkspaceVariables {
    pub(super) fn delete_profile(
        &mut self,
        index: usize,
    ) -> project::Result<Vec<WorkspaceProfile>> {
        if index > self.profiles.len() - 1 {
            warn!(
                "Failed to delete profile at row: {} (max: {})",
                index,
                self.profiles.len() - 1
            );
            return Err(ProjectError::ProfileNotFound.into());
        }
        self.profiles.remove(index);
        Ok(self.profiles.clone())
    }

    pub(super) fn add_profile(
        &mut self,
        index: Option<usize>,
        name: &str,
    ) -> Vec<WorkspaceProfile> {
        let name = self.next_profile_name(name);
        let new_profile = WorkspaceProfile {
            id: Uuid::new_v4(),
            name: SharedString::new(name),
            description: Default::default(),
        };
        if let Some(index) = index {
            if index > self.profiles.len() - 1 {
                self.profiles.push(new_profile);
            } else {
                self.profiles.insert(index, new_profile);
            }
        } else {
            self.profiles.push(new_profile);
        }
        self.profiles.clone()
    }

    pub(super) fn duplicate_profile(
        &mut self,
        index: usize,
    ) -> project::Result<Vec<WorkspaceProfile>> {
        let Some(profile) = self.profiles.get(index) else {
            warn!("Failed to duplicate profile at index: {}", index);
            return Err(ProjectError::ProfileNotFound.into());
        };
        let mut duplicated_profile = profile.clone();
        duplicated_profile.id = Uuid::new_v4();
        duplicated_profile.name = format!("{}_copy", &profile.name).into();

        let index = index + 1;
        if index >= self.profiles.len() {
            self.profiles.push(duplicated_profile);
        } else {
            self.profiles.insert(index, duplicated_profile);
        }
        Ok(self.profiles.clone())
    }

    pub(super) fn update_profile(
        &mut self,
        updated_profile: &WorkspaceProfile,
    ) -> project::Result<Vec<WorkspaceProfile>> {
        let Some(profile) = self
            .profiles
            .iter_mut()
            .find(|p| p.id == updated_profile.id)
        else {
            warn!(
                "Attempt to edit non existing profile with id: {}",
                updated_profile.id
            );
            return Err(ProjectError::ProfileNotFound.into());
        };

        if updated_profile != profile {
            profile.name = updated_profile.name.clone();
            profile.description = updated_profile.description.clone();
        }
        Ok(self.profiles.clone())
    }

    pub(crate) fn move_profile(
        &mut self,
        from_ix: usize,
        to_ix: usize,
    ) -> project::Result<Vec<WorkspaceProfile>> {
        let max_ix = self.profiles.len() - 1;
        if from_ix == to_ix || from_ix > max_ix || to_ix > max_ix {
            return Err(ProjectError::ProfileNotFound.into());
        }
        let profile_to_move = self.profiles.remove(from_ix);
        self.profiles.insert(to_ix, profile_to_move);
        Ok(self.profiles.clone())
    }

    pub(super) fn from_file(file_vars: &Variables) -> Self {
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

    pub(super) fn to_file(&self) -> Variables {
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

    fn next_profile_name(&mut self, name: &str) -> String {
        let mut final_name = name.to_string();
        {
            let mut i = 1;
            loop {
                if self.profiles.iter().any(|p| p.name == final_name) {
                    final_name = format!("{}_{}", name, i);
                    i += 1;
                } else {
                    break;
                }
            }
        }
        final_name
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
