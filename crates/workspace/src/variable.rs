use crate::error::ProjectError;
use crate::project;
use gpui_kit::component::select::SelectItem;
use gpui_kit::{Context, EventEmitter, SharedString};
use ki_project::{FileProfile, FileProjectVariables, FileVariable, FileVariableKind};
use ki_utils::ui::next_available_name;
use log::warn;
use project::Result;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum VariableKind {
  Text,
  PasswordClear,
  PasswordEncrypt,
}

impl From<FileVariableKind> for VariableKind {
  fn from(value: FileVariableKind) -> Self {
    match value {
      FileVariableKind::Text => VariableKind::Text,
      FileVariableKind::PasswordClear => VariableKind::PasswordClear,
      FileVariableKind::PasswordEncrypt => VariableKind::PasswordEncrypt,
    }
  }
}

impl From<VariableKind> for FileVariableKind {
  fn from(value: VariableKind) -> Self {
    match value {
      VariableKind::Text => FileVariableKind::Text,
      VariableKind::PasswordClear => FileVariableKind::PasswordClear,
      VariableKind::PasswordEncrypt => FileVariableKind::PasswordEncrypt,
    }
  }
}

impl Display for VariableKind {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      VariableKind::Text => f.write_str("Plain"),
      VariableKind::PasswordClear => f.write_str("Password (Clear)"),
      VariableKind::PasswordEncrypt => f.write_str("Password (Encrypted)"),
    }
  }
}

impl SelectItem for VariableKind {
  type Value = VariableKind;

  fn title(&self) -> SharedString {
    SharedString::new(format!("{self}"))
  }

  fn value(&self) -> &Self::Value {
    self
  }
}

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
      return Err(ProjectError::ProfileNotFound);
    }
    self.profiles.remove(index);
    cx.emit(ProjectVariablesEvent::ProfilesChanged);
    Ok(())
  }

  /// Duplicates a profile, attributing a new id and copying all fields. Name is appended with *_X* where X is an integer.
  ///
  /// The copy is placed right after the original
  /// # Result
  /// Returns a [`ProjectError::ProfileNotFound`] if the row is out of bound
  /// # Events
  /// Emits a [`ProjectVariablesEvent::ProfilesChanged`] if the profile
  pub fn duplicate_profile(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
    let Some(profile) = self.profiles.get(index) else {
      warn!("Failed to duplicate profile at index: {}", index);
      return Err(ProjectError::ProfileNotFound);
    };
    let mut duplicated_profile = profile.clone();
    duplicated_profile.id = Uuid::new_v4();
    duplicated_profile.name = next_available_name(&profile.name, &mut self.profiles.iter().map(|p| &p.name));

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
  pub fn move_profile(&mut self, from_ix: usize, to_ix: usize, cx: &mut Context<Self>) -> Result<()> {
    if from_ix == to_ix {
      return Ok(());
    }
    let max_ix = self.profiles.len() - 1;
    if from_ix == to_ix || from_ix > max_ix || to_ix > max_ix {
      return Err(ProjectError::ProfileNotFound);
    }
    let profile_to_move = self.profiles.remove(from_ix);
    self.profiles.insert(to_ix, profile_to_move);
    cx.emit(ProjectVariablesEvent::ProfilesChanged);
    Ok(())
  }

  pub fn profile_infos(&self) -> Vec<ProfileInfo> {
    self
      .profiles
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
  pub fn update_profile(&mut self, updated_profile: &ProfileInfo, cx: &mut Context<Self>) -> Result<()> {
    let Some(profile) = self.profiles.iter_mut().find(|p| p.id == updated_profile.id) else {
      warn!("Attempt to edit non existing profile with id: {}", updated_profile.id);
      return Err(ProjectError::ProfileNotFound);
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
      overrides: HashMap::new(),
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
      return Err(ProjectError::VariableNotFound);
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
      return Err(ProjectError::VariableNotFound);
    };
    let mut duplicated_var = var.clone();
    duplicated_var.id = Uuid::new_v4();
    duplicated_var.name = next_available_name(&var.name, &mut self.references.iter().map(|p| &p.name));

    let index = index + 1;
    if index >= self.references.len() {
      self.references.push(duplicated_var);
    } else {
      self.references.insert(index, duplicated_var);
    }
    cx.emit(ProjectVariablesEvent::VariablesChanged);
    Ok(())
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
    let Some(var) = self.references.iter_mut().find(|p| p.id == updated_variable.id) else {
      warn!("Attempt to edit non existing variable with id: {}", updated_variable.id);
      return Err(ProjectError::VariableNotFound);
    };

    if updated_variable == var {
      if let Some(profile_id) = profile_id
        && self.profiles.iter().any(|p| p.id == profile_id)
      {
        var.overrides.remove(&profile_id);
        cx.emit(ProjectVariablesEvent::VariablesChanged);
      }
      return Ok(());
    }

    var.name = updated_variable.name.clone();
    var.description = updated_variable.description.clone();
    var.kind = updated_variable.kind;
    if updated_variable.value != var.value
      && let Some(profile_id) = profile_id
      && self.profiles.iter().any(|p| p.id == profile_id)
    {
      var.overrides.insert(profile_id, updated_variable.value.clone());
    }

    cx.emit(ProjectVariablesEvent::VariablesChanged);
    Ok(())
  }

  pub fn move_variable(&mut self, from_ix: usize, to_ix: usize, cx: &mut Context<Self>) -> Result<()> {
    if from_ix == to_ix {
      return Ok(());
    }
    let max_ix = self.references.len() - 1;
    if from_ix == to_ix || from_ix > max_ix || to_ix > max_ix {
      return Err(ProjectError::VariableNotFound);
    }
    let var_to_move = self.references.remove(from_ix);
    self.references.insert(to_ix, var_to_move);
    cx.emit(ProjectVariablesEvent::VariablesChanged);
    Ok(())
  }

  ///// CONVERSION
  pub(super) fn from_file(file_vars: FileProjectVariables) -> Self {
    let profiles = file_vars.profiles.iter().map(Profile::from_file).collect::<Vec<Profile>>();
    let profile_ids: Vec<Uuid> = profiles.iter().map(|p| p.id).collect();
    let variables: Vec<VariableReference> = file_vars
      .variables
      .into_iter()
      .map(|file_var| VariableReference::from_file(file_var, &profile_ids))
      .collect();

    ProjectVariables {
      references: variables,
      profiles,
    }
  }

  pub(super) fn to_file(&self) -> FileProjectVariables {
    let variables = self.references.iter().map(|p| p.to_file()).collect();
    let profiles = self.profiles.iter().map(|p| p.to_file()).collect();

    FileProjectVariables { variables, profiles }
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
  overrides: HashMap<Uuid, SharedString>,
}

impl VariableReference {
  fn from_file(file_vars: FileVariable, profiles: &[Uuid]) -> Self {
    Self {
      id: file_vars.id,
      name: SharedString::new(&file_vars.name),
      description: SharedString::new(&file_vars.description),
      kind: file_vars.kind.into(),
      value: SharedString::new(&file_vars.value),
      overrides: file_vars
        .overrides
        .into_iter()
        .filter(|(profile_id, _)| profiles.contains(profile_id))
        .map(|(profile_id, value)| (profile_id, SharedString::new(value)))
        .collect(),
    }
  }

  fn to_file(&self) -> FileVariable {
    FileVariable {
      id: self.id,
      name: self.name.to_string(),
      description: self.description.to_string(),
      kind: self.kind.into(),
      value: self.value.to_string(),
      overrides: self
        .overrides
        .iter()
        .map(|(profile_id, value)| (*profile_id, value.to_string()))
        .collect(),
    }
  }

  pub fn find_override(&self, profile_id: &Uuid) -> Option<SharedString> {
    self
      .overrides
      .iter()
      .find_map(|(id, value)| if id == profile_id { Some(value.clone()) } else { None })
  }

  pub fn revert_override(&mut self, profile_id: Uuid, cx: &mut Context<ProjectVariables>) {
    if self.overrides.remove(&profile_id).is_some() {
      cx.emit(ProjectVariablesEvent::VariablesChanged);
    }
  }
}

#[derive(Eq, PartialEq, Clone, Debug)]
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

  fn to_file(&self) -> FileProfile {
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
