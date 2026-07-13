use crate::workspace::error::ProjectError;
use crate::workspace::project;
use gpui::{Context, EventEmitter, SharedString};
use gpui_component::select::SelectItem;
use ki_project::{FileProfile, FileProjectVariables, FileVariable, VariableKind};
use log::warn;
use project::Result;
use std::cmp::Ordering;
use std::collections::HashMap;
use uuid::Uuid;

///// WORKSPACE PROJECT EVENTS /////
pub enum ProjectVariablesEvent {
    /// Emitted when the list has been modified (profile added, removed, modified...)
    ProfilesChanged,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectVariables {
    pub references: Vec<VariableReference>,
    pub profiles: Vec<Profile>,
}

impl ProjectVariables {
    /// Adds a profile with the given name
    pub fn add_profile(
        &mut self,
        index: Option<usize>,
        name: &str,
        cx: &mut Context<Self>,
    ) -> Vec<Profile> {
        let name = self.next_profile_name(name);
        let new_profile = Profile {
            id: Uuid::new_v4(),
            name: SharedString::new(name),
            description: Default::default(),
            effective_values: Default::default(),
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
        cx.emit(ProjectVariablesEvent::ProfilesChanged);
        self.profiles.clone()
    }

    /// Deletes a profile a returns a copy of the new list and the removed profile, for further processing
    /// # Result
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
    pub fn delete_profile(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
        if index > self.profiles.len() - 1 {
            warn!(
                "Failed to delete profile at row: {} (max: {})",
                index,
                self.profiles.len() - 1
            );
            return Err(ProjectError::ProfileNotFound.into());
        }
        self.profiles.remove(index);
        cx.emit(ProjectVariablesEvent::ProfilesChanged);
        Ok(())
    }

    /// Duplicates a profile, attributing a new id and copying all fields. Name is appended with *_copy*.
    ///
    /// The copy is placed right after the original
    /// # Result
    /// Returns a copy of the new profiles list
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the row is out of bound
    pub fn duplicate_profile(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
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
        cx.emit(ProjectVariablesEvent::ProfilesChanged);
        Ok(())
    }

    /// Moves a profile from one position to another
    ///
    /// # Result
    /// Returns a copy of the new profiles list
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if both indexes are identical, or if either one of them is out of bound
    pub fn move_profile(
        &mut self,
        from_ix: usize,
        to_ix: usize,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let max_ix = self.profiles.len() - 1;
        if from_ix == to_ix || from_ix > max_ix || to_ix > max_ix {
            return Err(ProjectError::ProfileNotFound.into());
        }
        let profile_to_move = self.profiles.remove(from_ix);
        self.profiles.insert(to_ix, profile_to_move);
        cx.emit(ProjectVariablesEvent::ProfilesChanged);
        Ok(())
    }

    /// Merges a profile attributes
    ///
    /// # Result
    /// Returns a copy of the new profiles list if it was modified, else *None*
    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the profile could not be found by its id
    pub fn update_profile(
        &mut self,
        updated_profile: &ProfileInfo,
        cx: &mut Context<Self>,
    ) -> Result<()> {
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
            cx.emit(ProjectVariablesEvent::ProfilesChanged);
        }
        Ok(())
    }

    pub(super) fn from_file(file_vars: &FileProjectVariables) -> Self {
        let variables: Vec<VariableReference> = file_vars
            .variables
            .iter()
            .map(|var| VariableReference::from_file(var))
            .collect();
        let variable_ref: HashMap<Uuid, SharedString> = variables
            .iter()
            .map(|var| (var.id, var.value.clone()))
            .collect();
        let profiles = file_vars
            .profiles
            .iter()
            .map(|file_profile| Profile::from_file(file_profile, &variable_ref))
            .collect::<Vec<Profile>>();

        ProjectVariables {
            references: variables,
            profiles,
        }
    }

    pub(super) fn to_file(&self) -> FileProjectVariables {
        let variables = self.references.iter().map(|p| p.to_file()).collect();
        let profiles = self.profiles.iter().map(|p| p.to_file()).collect();

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

impl EventEmitter<ProjectVariablesEvent> for ProjectVariables {}

#[derive(Eq, Clone, Debug)]
pub struct VariableReference {
    pub id: Uuid,
    pub name: SharedString,
    pub description: SharedString,
    pub kind: VariableKind,
    pub value: SharedString,
}

impl VariableReference {
    fn from_file(file_profile: &FileVariable) -> Self {
        Self {
            id: file_profile.id,
            name: SharedString::new(&file_profile.name),
            description: SharedString::new(&file_profile.description),
            kind: file_profile.kind,
            value: SharedString::new(&file_profile.value),
        }
    }

    fn to_file(&self) -> FileVariable {
        FileVariable {
            id: self.id,
            name: self.name.to_string(),
            description: self.description.to_string(),
            kind: self.kind,
            value: self.value.to_string(),
        }
    }
}

impl PartialEq for VariableReference {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Eq, Clone, Debug)]
pub struct Profile {
    pub id: Uuid,
    pub name: SharedString,
    pub description: SharedString,
    pub effective_values: HashMap<Uuid, SharedString>,
}

impl Profile {
    fn from_file(
        file_profile: &FileProfile,
        variable_references: &HashMap<Uuid, SharedString>,
    ) -> Self {
        let effective_values = file_profile
            .overrides
            .iter()
            .filter_map(|(id, value)| {
                variable_references.get(id).map(|ref_value| {
                    if ref_value == value {
                        None
                    } else {
                        Some((*id, SharedString::new(value)))
                    }
                })?
            })
            .collect();
        Self {
            id: file_profile.id,
            name: SharedString::new(&file_profile.name),
            description: SharedString::new(&file_profile.description),
            effective_values,
        }
    }

    fn to_file(&self) -> FileProfile {
        let overrides = self
            .effective_values
            .iter()
            .map(|(id, value)| (id.clone(), value.to_string()))
            .collect();

        FileProfile {
            id: self.id,
            name: self.name.to_string(),
            description: self.description.to_string(),
            overrides,
        }
    }
}

impl PartialEq for Profile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
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

#[derive(Clone, Debug)]
pub struct ProfileInfo {
    pub id: Uuid,
    pub name: SharedString,
    pub description: SharedString,
}

impl PartialEq<Profile> for ProfileInfo {
    fn eq(&self, other: &Profile) -> bool {
        self.id == other.id && self.name == other.name && self.description == other.description
    }
}
