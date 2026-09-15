use ki_project::ProjectFile;
use std::path::Path;

#[test]
fn test_load_file() {
  let path = Path::new("../../data/example_project.kpr");
  ProjectFile::load(path).expect("failed to load project");
}
