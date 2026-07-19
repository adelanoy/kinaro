use ki_utils::ui::next_available_name;
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

///// PROJECT VARIABLES EVENTS /////
pub enum ProjectVariablesEvent {
    /// Emitted when the profile list has been modified (profile added, removed, modified...)
    ProfilesChanged,
    /// Emitted when the reference variables has been modified (added, removed, modified...)
    VariablesChanged,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectVariables {
    pub references: Vec<VariableReference>,
    pub profiles: Vec<Profile>,
}

impl ProjectVariables {
    ///// PROFILES

    /// Adds a profile with the given name
    /// # Events
    /// Emits a [`ProjectVariablesEvent::ProfilesChanged`]
    pub fn add_profile(&mut self, index: Option<usize>, name: &str, cx: &mut Context<Self>) {
        let name = next_available_name(name, &mut self.profiles.iter().map(|p| &p.name));
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
    }

    /// Deletes a profile
    /// # Result
    /// Returns a [`ProjectError::ProfileNotFound`] if the index is out of bound
    /// # Events
    /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile was deleted
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

    /// Duplicates a profile, attributing a new id and copying all fields. Name is appended with *_X* where X is an integer.
    ///
    /// The copy is placed right after the original
    /// # Result    ///
    /// Returns a [`ProjectError::ProfileNotFound`] if the row is out of bound
    /// # Events
    /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile
    pub fn duplicate_profile(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
        let Some(profile) = self.profiles.get(index) else {
            warn!("Failed to duplicate profile at index: {}", index);
            return Err(ProjectError::ProfileNotFound.into());
        };
        let mut duplicated_profile = profile.clone();
        duplicated_profile.id = Uuid::new_v4();
        duplicated_profile.name =
            next_available_name(&profile.name, &mut self.references.iter().map(|p| &p.name));

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

    pub fn profile_infos(&self) -> Vec<ProfileInfo> {
        self.profiles
            .iter()
            .map(|p| ProfileInfo {
                id: p.id,
                name: p.name.clone(),
                description: p.description.clone(),
            })
            .collect()
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

    ///// VARIABLES
    pub fn add_variable(&mut self, index: Option<usize>, name: &str, cx: &mut Context<Self>) {
        let name = next_available_name(name, &mut self.references.iter().map(|p| &p.name));
        let new_var = VariableReference {
            id: Uuid::new_v4(),
            name: SharedString::new(name),
            description: Default::default(),
            kind: VariableKind::Text,
            value: SharedString::new(""),
        };
        if let Some(index) = index {
            if index > self.references.len() - 1 {
                self.references.push(new_var);
            } else {
                self.references.insert(index, new_var);
            }
        } else {
            self.references.push(new_var);
        }
        cx.emit(ProjectVariablesEvent::VariablesChanged);
    }

    /// Deletes a variable
    /// # Result
    /// Returns a [`ProjectError::VariableNotFound`] if the index is out of bound
    /// # Events
    /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the profile was deleted
    pub fn delete_variable(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
        if index > self.references.len() - 1 {
            warn!(
                "Failed to delete variable at row: {} (max: {})",
                index,
                self.references.len() - 1
            );
            return Err(ProjectError::VariableNotFound.into());
        }
        self.references.remove(index);
        cx.emit(ProjectVariablesEvent::VariablesChanged);
        Ok(())
    }

    /// Duplicates a variable, attributing a new id and copying all fields. Name is appended with *_X* where X is an integer.
    ///
    /// The copy is placed right after the original
    /// # Result
    /// Returns a [`ProjectError::VariableNotFound`] if the row is out of bound
    /// # Events
    /// Emits a [`ProjectVariablesEvent::VariablesChanged`] if the profile
    pub fn duplicate_variable(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
        let Some(var) = self.references.get(index) else {
            warn!("Failed to duplicate variable at index: {}", index);
            return Err(ProjectError::VariableNotFound.into());
        };
        let mut duplicated_var = var.clone();
        duplicated_var.id = Uuid::new_v4();
        duplicated_var.name =
            next_available_name(&var.name, &mut self.references.iter().map(|p| &p.name));

        let index = index + 1;
        if index >= self.references.len() {
            self.references.push(duplicated_var);
        } else {
            self.references.insert(index, duplicated_var);
        }
        cx.emit(ProjectVariablesEvent::VariablesChanged);
        Ok(())
    }

    pub fn find_override(&self, var_id: Uuid, profile_id: Uuid) -> Option<SharedString> {
        let profile = self.profiles.iter().find(|p| p.id == profile_id)?;
        profile
            .effective_values
            .get(&var_id)
            .map(|value| value.clone())
    }

    pub fn revert_profile(
        &mut self,
        var_id: Uuid,
        profile_id: Uuid,
        cx: &mut Context<ProjectVariables>,
    ) {
        if let Some(profile) = self.profiles.iter_mut().find(|p| p.id == profile_id) {
            if profile.effective_values.remove(&var_id).is_some() {
                cx.emit(ProjectVariablesEvent::VariablesChanged);
            }
        }
    }

    /// Merges a variable attributes.
    ///
    /// If a profile id is provided, the profile's override value is updated, else the reference value is updated
    /// # Result
    ///
    /// Returns a [`ProjectError::VariableNotFound`] if the profile could not be found by its id
    pub fn update_variable(
        &mut self,
        profile_id: Option<Uuid>,
        updated_variable: &VariableReference,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let Some(var) = self
            .references
            .iter_mut()
            .find(|p| p.id == updated_variable.id)
        else {
            warn!(
                "Attempt to edit non existing variable with id: {}",
                updated_variable.id
            );
            return Err(ProjectError::VariableNotFound.into());
        };

        if updated_variable != var {
            var.name = updated_variable.name.clone();
            var.description = updated_variable.description.clone();
            var.kind = updated_variable.kind;
            cx.emit(ProjectVariablesEvent::VariablesChanged);
        }
        if let Some(profile_id) = profile_id {
            if let Some(profile) = self.profiles.iter_mut().find(|p| p.id == profile_id) {
                profile
                    .effective_values
                    .insert(updated_variable.id, updated_variable.value.clone());
            }
        } else {
            var.value = updated_variable.value.clone();
        }
        Ok(())
    }

    ///// CONVERSION
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
}

impl EventEmitter<ProjectVariablesEvent> for ProjectVariables {}

#[derive(Eq, PartialEq, Clone, Debug)]
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

#[derive(Eq, PartialEq, Clone, Debug)]
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

impl PartialOrd for Profile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
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

impl PartialEq for ProfileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl SelectItem for ProfileInfo {
    type Value = Uuid;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}
