use crate::workspace::error::ProjectError;
use crate::workspace::project;
use gpui::SharedString;
use gpui_component::select::SelectItem;
use ki_project::{FileProfile, FileProjectVariables, FileVariable, VariableKind};
use log::warn;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectVariables {
    pub variables: Vec<Variable>,
    pub profiles: Vec<Profile>,
}

impl ProjectVariables {
    /// Adds a profile with the given name
    pub(super) fn add_profile(&mut self, index: Option<usize>, name: &str) -> Vec<Profile> {
        let name = self.next_profile_name(name);
        let new_profile = Profile {
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

    /// Deletes a profile a returns a copy of the new list and the removed profile, for further processing
    /// # Result
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
    pub(super) fn delete_profile(
        &mut self,
        index: usize,
    ) -> project::Result<(Vec<Profile>, Profile)> {
        if index > self.profiles.len() - 1 {
            warn!(
                "Failed to delete profile at row: {} (max: {})",
                index,
                self.profiles.len() - 1
            );
            return Err(ProjectError::ProfileNotFound.into());
        }
        let removed_profile = self.profiles.remove(index);
        Ok((self.profiles.clone(), removed_profile))
    }

    /// Duplicates a profile, attributing a new id and copying all fields. Name is appended with *_copy*.
    ///
    /// The copy is placed right after the original
    /// # Result
    /// Returns a copy of the new profiles list
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the row is out of bound
    pub(super) fn duplicate_profile(&mut self, index: usize) -> project::Result<Vec<Profile>> {
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

    /// Moves a profile from one position to another
    ///
    /// # Result
    /// Returns a copy of the new profiles list
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if both indexes are identical, or if either one of them is out of bound
    pub(crate) fn move_profile(
        &mut self,
        from_ix: usize,
        to_ix: usize,
    ) -> project::Result<Vec<Profile>> {
        let max_ix = self.profiles.len() - 1;
        if from_ix == to_ix || from_ix > max_ix || to_ix > max_ix {
            return Err(ProjectError::ProfileNotFound.into());
        }
        let profile_to_move = self.profiles.remove(from_ix);
        self.profiles.insert(to_ix, profile_to_move);
        Ok(self.profiles.clone())
    }

    /// Merges a profile attributes
    ///
    /// # Result
    /// Returns a copy of the new profiles list if it was modified, else *None*
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the profile could not be found by its id
    pub(super) fn update_profile(
        &mut self,
        updated_profile: &Profile,
    ) -> project::Result<Option<Vec<Profile>>> {
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
            Ok(Some(self.profiles.clone()))
        } else {
            Ok(None)
        }
    }

    pub(super) fn from_file(file_vars: &FileProjectVariables) -> Self {
        let profiles = file_vars
            .profiles
            .iter()
            .map(|p| Profile::from_file(p))
            .collect::<Vec<Profile>>();

        let variables = file_vars
            .variables
            .iter()
            .map(|file_var| {
                let overrides = profiles
                    .iter()
                    .map(|profile| (profile.id, file_var.overrides.get(&profile.id).cloned()))
                    .collect::<HashMap<Uuid, Option<String>>>();
                Variable {
                    id: file_var.id,
                    name: SharedString::new(&file_var.name),
                    description: SharedString::new(&file_var.description),
                    kind: file_var.kind,
                    reference_value: SharedString::new(&file_var.value),
                    effective_values: overrides,
                }
            })
            .collect::<Vec<Variable>>();

        ProjectVariables {
            variables,
            profiles,
        }
    }

    pub(super) fn to_file(&self) -> FileProjectVariables {
        let profiles = self
            .profiles
            .iter()
            .map(|p| p.get_file())
            .collect::<BTreeSet<FileProfile>>();

        let mut variables = vec![];
        for workspace_var in &self.variables {
            let workspace_var = workspace_var;
            let overrides = workspace_var
                .effective_values
                .iter()
                .filter(|(_, value)| value.is_some())
                .map(|(id, value)| (*id, value.clone().unwrap()))
                .collect::<HashMap<Uuid, String>>();
            variables.push(FileVariable {
                id: workspace_var.id,
                name: workspace_var.name.to_string(),
                description: workspace_var.description.to_string(),
                kind: workspace_var.kind,
                value: workspace_var.reference_value.to_string(),
                overrides,
            });
        }

        FileProjectVariables {
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
pub struct Variable {
    pub id: Uuid,
    pub name: SharedString,
    pub description: SharedString,
    pub kind: VariableKind,
    pub reference_value: SharedString,
    pub effective_values: HashMap<Uuid, Option<String>>,
}

#[derive(Eq, PartialEq, Ord, Clone, Debug)]
pub struct Profile {
    pub id: Uuid,
    pub name: SharedString,
    pub description: SharedString,
}

impl Profile {
    fn from_file(file_profile: &FileProfile) -> Self {
        Self {
            id: file_profile.id,
            name: SharedString::new(&file_profile.name),
            description: SharedString::new(&file_profile.description),
        }
    }

    fn get_file(&self) -> FileProfile {
        FileProfile {
            id: self.id,
            name: self.name.to_string(),
            description: self.description.to_string(),
        }
    }
}

impl PartialOrd for Profile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl SelectItem for Profile {
    type Value = Uuid;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}
