use crate::errors::{ProjectError, TokenError};
use common_path::common_path_all;
use moon_utils::glob;
use moon_utils::path::expand_root_path;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FileGroup {
    pub files: Vec<String>,

    pub id: String,
}

impl FileGroup {
    pub fn new(id: &str, files: Vec<String>) -> FileGroup {
        FileGroup {
            files,
            id: id.to_owned(),
        }
    }

    pub fn merge(&mut self, files: Vec<String>) {
        // Local files should always override global
        self.files = files;
    }

    /// Returns the file group as an expanded list of relative directory paths.
    /// If a glob is detected, it will aggregate all directories found.
    pub fn dirs(
        &self,
        workspace_root: &Path,
        project_root: &Path,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        self.walk(true, workspace_root, project_root)
    }

    /// Returns the file group as an expanded list of relative file paths.
    /// If a glob is detected, it will aggregate all files found.
    pub fn files(
        &self,
        workspace_root: &Path,
        project_root: &Path,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        self.walk(false, workspace_root, project_root)
    }

    /// Returns the file group as a list of file globs (as-is),
    /// relative to the project root.
    pub fn globs(
        &self,
        workspace_root: &Path,
        project_root: &Path,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        let mut globs = vec![];

        for file in &self.files {
            if glob::is_glob(file) {
                globs.push(expand_root_path(file, workspace_root, project_root));
            }
        }

        if globs.is_empty() {
            return Err(ProjectError::Token(TokenError::NoGlobs(self.id.to_owned())));
        }

        Ok(globs)
    }

    /// Returns the file group reduced down to the lowest common directory.
    /// If the reduced directories is not =1, the project root "." will be returned.
    pub fn root(&self, project_root: &Path) -> Result<PathBuf, ProjectError> {
        let dirs = self.dirs(project_root, project_root)?; // Workspace not needed!

        if !dirs.is_empty() {
            let paths: Vec<&Path> = dirs
                .iter()
                .filter(|d| d.starts_with(&project_root))
                .map(|d| d.strip_prefix(&project_root).unwrap())
                .collect();
            let common_dir = common_path_all(paths);

            if let Some(dir) = common_dir {
                return Ok(project_root.join(dir));
            }
        }

        // Too many dirs or no dirs, so return the project root
        Ok(project_root.to_owned())
    }

    fn walk(
        &self,
        is_dir: bool,
        workspace_root: &Path,
        project_root: &Path,
    ) -> Result<Vec<PathBuf>, ProjectError> {
        let mut list = vec![];

        for file in &self.files {
            if glob::is_glob(file) {
                let root = if file.starts_with('/') {
                    workspace_root
                } else {
                    project_root
                };

                for path in glob::walk(root, &[file.clone()]) {
                    let allowed = if is_dir {
                        path.is_dir()
                    } else {
                        path.is_file()
                    };

                    if allowed {
                        list.push(path);
                    }
                }
            } else {
                let path = expand_root_path(file, workspace_root, project_root);

                let allowed = if is_dir {
                    path.is_dir()
                } else {
                    path.is_file()
                };

                if allowed {
                    list.push(path.to_owned());
                }
            }
        }

        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_utils::string_vec;
    use moon_utils::test::get_fixtures_dir;

    mod merge {
        use super::*;

        #[test]
        fn overwrites() {
            let mut file_group = FileGroup::new("id", string_vec!["**/*"]);

            file_group.merge(string_vec!["*"]);

            assert_eq!(file_group.files, string_vec!["*"]);
        }
    }

    mod dirs {
        use super::*;

        #[test]
        fn returns_all_dirs() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let file_group = FileGroup::new("id", string_vec!["**/*"]);

            assert_eq!(
                file_group.dirs(&workspace_root, &project_root).unwrap(),
                vec![project_root.join("dir"), project_root.join("dir/subdir")]
            );
        }

        #[test]
        fn doesnt_return_files() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let file_group = FileGroup::new("id", string_vec!["file.ts"]);
            let result: Vec<PathBuf> = vec![];

            assert_eq!(
                file_group.dirs(&workspace_root, &project_root).unwrap(),
                result
            );
        }
    }

    mod files {
        use super::*;

        #[test]
        fn returns_all_files() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let file_group = FileGroup::new(
                "id",
                string_vec![
                    // Globs
                    "**/*.{ts,tsx}",
                    "/*.json",
                    // Literals
                    "README.md",
                    "/README.md"
                ],
            );

            let mut files = file_group.files(&workspace_root, &project_root).unwrap();
            files.sort();

            assert_eq!(
                files,
                vec![
                    workspace_root.join("README.md"),
                    project_root.join("README.md"),
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts"),
                    project_root.join("file.ts"),
                    workspace_root.join("package.json"),
                ]
            );
        }

        #[test]
        fn doesnt_return_dirs() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let file_group = FileGroup::new("id", string_vec!["dir"]);
            let result: Vec<PathBuf> = vec![];

            assert_eq!(
                file_group.files(&workspace_root, &project_root).unwrap(),
                result
            );
        }
    }

    mod globs {
        use super::*;

        #[test]
        fn returns_only_globs() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let file_group =
                FileGroup::new("id", string_vec!["**/*", "*.rs", "file.ts", "dir", "/*.js"]);

            assert_eq!(
                file_group.globs(&workspace_root, &project_root).unwrap(),
                vec![
                    project_root.join("**/*"),
                    project_root.join("*.rs"),
                    workspace_root.join("*.js")
                ]
            );
        }
    }

    mod root {
        use super::*;

        #[test]
        fn returns_lowest_dir() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let file_group = FileGroup::new("id", string_vec!["**/*"]);

            assert_eq!(
                file_group.root(&project_root).unwrap(),
                project_root.join("dir")
            );
        }

        #[test]
        fn returns_root_when_many() {
            let workspace_root = get_fixtures_dir("projects");
            let file_group = FileGroup::new("id", string_vec!["**/*"]);

            assert_eq!(file_group.root(&workspace_root).unwrap(), workspace_root);
        }

        #[test]
        fn returns_root_when_no_dirs() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let file_group = FileGroup::new("id", string_vec![]);

            assert_eq!(file_group.root(&project_root).unwrap(), project_root);
        }
    }
}
