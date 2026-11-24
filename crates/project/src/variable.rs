use crate::error::ProjectValidationError;
use ki_utils::id::KId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[non_exhaustive]
pub struct FileProjectVariables {
  pub variables: Vec<FileVariable>,
  pub profiles: Vec<FileProfile>,
}

impl FileProjectVariables {
  /// Creates a new [`FileProjectVariables`], trimming the variables names and dropping the overrides of profiles that are
  /// not part of `profiles`
  pub fn new(mut variables: Vec<FileVariable>, profiles: Vec<FileProfile>) -> Self {
    let profiles_id: HashSet<KId> = profiles.iter().map(|p| p.id).collect();
    for var in variables.iter_mut() {
      var.name = var.name.trim().to_string();
      var.overrides.retain(|o, _| profiles_id.contains(o));
    }
    Self { variables, profiles }
  }

  /// Checks the consistency of the variables, returning every problem found (empty when valid): invalid or duplicated
  /// variable names, and overrides of unknown profiles
  pub(crate) fn check(&self) -> Vec<ProjectValidationError> {
    let mut errors = vec![];
    let profile_ids: HashSet<KId> = self.profiles.iter().map(|p| p.id).collect();
    let mut names: HashSet<&str> = HashSet::new();
    for var in &self.variables {
      if !ki_utils::is_variable_name_valid(&var.name) {
        errors.push(ProjectValidationError::InvalidVariableName(var.name.clone()));
      }
      if names.contains(var.name.as_str()) {
        errors.push(ProjectValidationError::DuplicateVariableName(var.name.clone()));
      }
      names.insert(&var.name);

      for override_id in var.overrides.keys() {
        if !profile_ids.contains(override_id) {
          errors.push(ProjectValidationError::InvalidOverride(var.name.clone(), *override_id));
        }
      }
    }
    errors
  }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct FileVariable {
  pub name: String,
  pub description: String,
  pub kind: FileVariableKind,
  pub value: String,
  /// BTreeMap of overridden values by profile IDs. BTreemap is used to prevent random ordering on save, even unmodified, triggering false
  ///positive diffs on project file
  pub overrides: BTreeMap<KId, String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
pub enum FileVariableKind {
  Text,
  PasswordClear,
  PasswordEncrypt,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct FileProfile {
  pub id: KId,
  pub name: String,
  pub description: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  fn profile(id: KId, name: &str) -> FileProfile {
    FileProfile {
      id,
      name: name.to_string(),
      description: String::new(),
    }
  }

  fn variable(name: &str, overrides: &[(KId, &str)]) -> FileVariable {
    FileVariable {
      name: name.to_string(),
      description: String::new(),
      kind: FileVariableKind::Text,
      value: "value".to_string(),
      overrides: overrides.iter().map(|(id, v)| (*id, v.to_string())).collect(),
    }
  }

  #[test]
  fn new_trims_names_and_drops_unknown_overrides() {
    let (known, unknown) = (KId::new(), KId::new());
    let vars = FileProjectVariables::new(
      vec![variable("  host ", &[(known, "a"), (unknown, "b")])],
      vec![profile(known, "dev")],
    );
    assert_eq!(vars.variables[0].name, "host");
    assert_eq!(vars.variables[0].overrides.keys().copied().collect::<Vec<_>>(), vec![known]);
  }

  #[test]
  fn check_accepts_valid_variables() {
    let dev = KId::new();
    let vars = FileProjectVariables::new(
      vec![variable("host", &[(dev, "a")]), variable("port", &[])],
      vec![profile(dev, "dev")],
    );
    assert!(vars.check().is_empty());
  }

  #[test]
  fn check_reports_invalid_names() {
    let vars = FileProjectVariables::new(vec![variable("my-var", &[]), variable("", &[])], vec![]);
    let errors = vars.check();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(matches!(&errors[0], ProjectValidationError::InvalidVariableName(name) if name == "my-var"));
    assert!(matches!(&errors[1], ProjectValidationError::InvalidVariableName(name) if name.is_empty()));
  }

  #[test]
  fn check_reports_duplicated_names() {
    let vars = FileProjectVariables::new(
      vec![variable("host", &[]), variable("port", &[]), variable("host", &[])],
      vec![],
    );
    let errors = vars.check();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(matches!(&errors[0], ProjectValidationError::DuplicateVariableName(name) if name == "host"));
  }

  #[test]
  fn check_reports_unknown_overrides() {
    let unknown = KId::new();
    // Built directly: `new` would strip the unknown override
    let vars = FileProjectVariables {
      variables: vec![variable("host", &[(unknown, "a")])],
      profiles: vec![profile(KId::new(), "dev")],
    };
    let errors = vars.check();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(matches!(&errors[0], ProjectValidationError::InvalidOverride(name, id) if name == "host" && *id == unknown));
  }

  #[test]
  fn check_reports_every_error() {
    let vars = FileProjectVariables {
      variables: vec![variable("a-b", &[(KId::new(), "x")]), variable("a-b", &[])],
      profiles: vec![],
    };
    // 2 invalid names, 1 duplicate, 1 unknown override
    assert_eq!(vars.check().len(), 4);
  }
}
