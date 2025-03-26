use crate::file_group_error::FileGroupError;
use common_path::common_path_all;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::InputPath;
use moon_feature_flags::glob_walk_with_options;
use serde::{Deserialize, Serialize};
use starbase_utils::glob::{self, GlobWalkOptions};
use std::path::Path;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct FileGroup {
    pub env: Vec<String>,

    pub files: Vec<WorkspaceRelativePathBuf>,

    pub globs: Vec<WorkspaceRelativePathBuf>,

    pub id: Id,
}

impl FileGroup {
    pub fn new<T>(id: T) -> miette::Result<FileGroup>
    where
        T: AsRef<str>,
    {
        Ok(FileGroup {
            env: vec![],
            files: vec![],
            globs: vec![],
            id: Id::new(id)?,
        })
    }

    pub fn new_with_source<T, I>(id: T, patterns: I) -> miette::Result<FileGroup>
    where
        T: AsRef<str>,
        I: IntoIterator<Item = WorkspaceRelativePathBuf>,
    {
        let mut file_group = FileGroup::new(id)?;

        for path in patterns {
            if glob::is_glob(&path) {
                file_group.globs.push(path);
            } else {
                file_group.files.push(path);
            }
        }

        Ok(file_group)
    }

    /// Add inputs (file paths, globs, or env) to the file group, while expanding to a
    /// workspace relative path based on the provided project source.
    pub fn add(&mut self, input: &InputPath, project_source: &str) -> miette::Result<()> {
        match input {
            InputPath::EnvVar(var) | InputPath::EnvVarGlob(var) => {
                self.env.push(var.to_owned());
            }
            InputPath::TokenFunc(_) | InputPath::TokenVar(_) => {
                return Err(FileGroupError::NoTokens(self.id.clone()).into());
            }
            _ => {
                let path = input.to_workspace_relative(project_source);

                if input.is_glob() {
                    self.globs.push(path);
                } else {
                    self.files.push(path);
                }
            }
        };

        Ok(())
    }

    /// Add multiple inputs into the input group.
    pub fn add_many(&mut self, inputs: &[InputPath], project_source: &str) -> miette::Result<()> {
        for input in inputs {
            self.add(input, project_source)?;
        }

        Ok(())
    }

    /// Return the file group as an expanded list of directory paths.
    /// If a glob is detected, it will aggregate all directories found.
    pub fn dirs(
        &self,
        workspace_root: &Path,
        loose_check: bool,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        self.walk(true, workspace_root, loose_check)
    }

    /// Return the list of environment variables.
    pub fn env(&self) -> miette::Result<&Vec<String>> {
        Ok(&self.env)
    }

    /// Return the file group as an expanded list of file paths.
    /// If a glob is detected, it will aggregate all files found.
    pub fn files(
        &self,
        workspace_root: &Path,
        loose_check: bool,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        self.walk(false, workspace_root, loose_check)
    }

    /// Return the file group as a list of file globs (as-is),
    /// relative to the project root.
    pub fn globs(&self) -> miette::Result<&Vec<WorkspaceRelativePathBuf>> {
        if self.globs.is_empty() {
            return Err(FileGroupError::MissingGlobs(self.id.clone()).into());
        }

        Ok(&self.globs)
    }

    /// Return the file group reduced down to the lowest common directory.
    /// If the reduced directories is not = 1, the project root "." will be returned.
    pub fn root<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        workspace_root: P,
        project_source: S,
    ) -> miette::Result<WorkspaceRelativePathBuf> {
        let dirs = self.dirs(workspace_root.as_ref(), false)?;
        let project_source = project_source.as_ref();

        if !dirs.is_empty() {
            let paths = dirs
                .iter()
                .filter_map(|d| d.strip_prefix(project_source).ok())
                .map(|d| Path::new(d.as_str()))
                .collect::<Vec<&Path>>();

            let common_dir = common_path_all(paths);

            if let Some(dir) = common_dir {
                return Ok(
                    WorkspaceRelativePathBuf::from(project_source).join(dir.to_str().unwrap())
                );
            }
        }

        Ok(".".into())
    }

    fn walk(
        &self,
        is_dir: bool,
        workspace_root: &Path,
        loose_check: bool,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut list = vec![];

        for path in &self.files {
            let file = path.to_path(workspace_root);
            let mut allowed = false;

            #[allow(clippy::if_same_then_else)]
            if is_dir && (file.is_dir() || loose_check && file.extension().is_none()) {
                allowed = true;
            } else if !is_dir && (file.is_file() || loose_check && file.extension().is_some()) {
                allowed = true;
            }

            if allowed {
                list.push(path.to_owned());
            }
        }

        if !self.globs.is_empty() {
            // Glob results are absolute paths!
            for path in glob_walk_with_options(
                workspace_root,
                &self.globs,
                GlobWalkOptions::default().cache(),
            )? {
                let allowed = if is_dir {
                    path.is_dir()
                } else {
                    path.is_file()
                };

                if allowed {
                    list.push(
                        WorkspaceRelativePathBuf::from_path(
                            path.strip_prefix(workspace_root).unwrap(),
                        )
                        .unwrap(),
                    );
                }
            }
        }

        list.sort();

        Ok(list)
    }
}

impl PartialEq for FileGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.files == other.files && self.globs == other.globs
    }
}
