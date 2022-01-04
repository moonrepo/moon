use crate::errors::{ProjectError, TokenError};
use common_path::common_path_all;
use globwalk::GlobWalkerBuilder;
use moon_utils::fs::is_glob;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FileGroup {
    pub files: Vec<String>,

    pub id: String,

    #[serde(skip)]
    project_root: PathBuf,
}

impl FileGroup {
    pub fn new(id: &str, files: Vec<String>, project_root: &Path) -> FileGroup {
        FileGroup {
            files,
            id: id.to_owned(),
            project_root: project_root.to_path_buf(),
        }
    }

    pub fn merge(&mut self, files: Vec<String>) {
        // Local files should always override global
        self.files = files;
    }

    /// Returns the file group as an expanded list of relative directory paths.
    /// If a glob is detected, it will aggregate all directories found.
    pub fn dirs(&self) -> Result<Vec<String>, ProjectError> {
        self.walk(true)
    }

    /// Returns the file group as an expanded list of relative file paths.
    /// If a glob is detected, it will aggregate all files found.
    pub fn files(&self) -> Result<Vec<String>, ProjectError> {
        self.walk(false)
    }

    /// Returns the file group as a list of file globs (as-is),
    /// relative to the project root.
    pub fn globs(&self) -> Result<Vec<String>, ProjectError> {
        let mut globs = vec![];

        for file in &self.files {
            if is_glob(file) {
                globs.push(file.to_owned())
            }
        }

        if globs.is_empty() {
            return Err(ProjectError::Token(TokenError::NoGlobs(self.id.to_owned())));
        }

        Ok(globs)
    }

    /// Returns the file group reduced down to the lowest common directory.
    /// If the reduced directories is not =1, the project root "." will be returned.
    pub fn root(&self) -> Result<String, ProjectError> {
        let dirs = self.dirs()?;

        if !dirs.is_empty() {
            let paths: Vec<&Path> = dirs.iter().map(Path::new).collect();
            let common_dir = common_path_all(paths);

            if let Some(dir) = common_dir {
                return Ok(String::from(dir.to_string_lossy()));
            }
        }

        // Too many dirs or no dirs, so return the project root
        Ok(String::from("."))
    }

    fn walk(&self, is_dir: bool) -> Result<Vec<String>, ProjectError> {
        let mut list = vec![];

        for file in &self.files {
            if is_glob(file) {
                let walker = GlobWalkerBuilder::from_patterns(&self.project_root, &[file])
                    .follow_links(false)
                    .build()?;

                for entry in walker {
                    let entry_path = entry.unwrap(); // Handle error?

                    let allowed = if is_dir {
                        entry_path.file_type().is_dir()
                    } else {
                        entry_path.file_type().is_file()
                    };

                    if allowed {
                        let entry_path = entry_path.into_path();
                        let entry_path = entry_path.strip_prefix(&self.project_root).unwrap();

                        list.push(String::from(entry_path.to_str().ok_or_else(|| {
                            ProjectError::InvalidUtf8File(String::from(
                                entry_path.to_string_lossy(),
                            ))
                        })?));
                    }
                }
            } else {
                let allowed = match fs::metadata(&self.project_root.join(file)) {
                    Ok(meta) => {
                        if is_dir {
                            meta.is_dir()
                        } else {
                            meta.is_file()
                        }
                    }
                    // Branch exists for logging
                    Err(_) => false,
                };

                if allowed {
                    list.push(file.to_owned());
                }
            }
        }

        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_utils::test::get_fixtures_dir;

    mod merge {
        use super::*;

        #[test]
        fn overwrites() {
            let root = get_fixtures_dir("base");
            let mut file_group =
                FileGroup::new("id", vec!["**/*".to_owned()], &root.join("files-and-dirs"));

            file_group.merge(vec!["*".to_owned()]);

            assert_eq!(file_group.files, vec!["*".to_owned()]);
        }
    }

    mod dirs {
        use super::*;

        #[test]
        fn returns_all_dirs() {
            let root = get_fixtures_dir("base");
            let file_group =
                FileGroup::new("id", vec!["**/*".to_owned()], &root.join("files-and-dirs"));

            assert_eq!(
                file_group.dirs().unwrap(),
                vec![String::from("dir"), String::from("dir/subdir")]
            );
        }

        #[test]
        fn doesnt_return_files() {
            let root = get_fixtures_dir("base");
            let file_group = FileGroup::new(
                "id",
                vec!["file.ts".to_owned()],
                &root.join("files-and-dirs"),
            );
            let result: Vec<String> = vec![];

            assert_eq!(file_group.dirs().unwrap(), result);
        }
    }

    mod files {
        use super::*;

        #[test]
        fn returns_all_files() {
            let root = get_fixtures_dir("base");
            let file_group = FileGroup::new(
                "id",
                vec!["**/*.{ts,tsx}".to_owned()],
                &root.join("files-and-dirs"),
            );

            assert_eq!(
                file_group.files().unwrap(),
                vec![
                    String::from("file.ts"),
                    String::from("dir/subdir/another.ts"),
                    String::from("dir/other.tsx"),
                ]
            );
        }

        #[test]
        fn doesnt_return_dirs() {
            let root = get_fixtures_dir("base");
            let file_group =
                FileGroup::new("id", vec!["dir".to_owned()], &root.join("files-and-dirs"));
            let result: Vec<String> = vec![];

            assert_eq!(file_group.files().unwrap(), result);
        }
    }

    mod globs {
        use super::*;

        #[test]
        fn returns_only_globs() {
            let root = get_fixtures_dir("base");
            let file_group = FileGroup::new(
                "id",
                vec![
                    "**/*".to_owned(),
                    "*.rs".to_owned(),
                    "file.ts".to_owned(),
                    "dir".to_owned(),
                ],
                &root.join("files-and-dirs"),
            );

            assert_eq!(
                file_group.globs().unwrap(),
                vec!["**/*".to_owned(), "*.rs".to_owned()]
            );
        }
    }

    mod root {
        use super::*;

        #[test]
        fn returns_lowest_dir() {
            let root = get_fixtures_dir("base");
            let file_group =
                FileGroup::new("id", vec!["**/*".to_owned()], &root.join("files-and-dirs"));

            assert_eq!(file_group.root().unwrap(), String::from("dir"));
        }

        #[test]
        fn returns_root_when_many() {
            let root = get_fixtures_dir("projects");
            let file_group = FileGroup::new("id", vec!["**/*".to_owned()], &root);

            assert_eq!(file_group.root().unwrap(), String::from("."));
        }

        #[test]
        fn returns_root_when_no_dirs() {
            let root = get_fixtures_dir("base");
            let file_group = FileGroup::new("id", vec![], &root);

            assert_eq!(file_group.root().unwrap(), String::from("."));
        }
    }
}
