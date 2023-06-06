use chrono::Local;
use moon_config::{CodeownersConfig, OwnersConfig, OwnersPaths, VcsProvider};
use starbase_utils::fs::{self, FsError};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct CodeownersGenerator {
    file: File,
    file_path: PathBuf,
    provider: VcsProvider,
}

impl CodeownersGenerator {
    pub fn new(workspace_root: &Path, provider: VcsProvider) -> Result<Self, FsError> {
        let file_path = workspace_root.join(match provider {
            VcsProvider::GitHub => ".github/CODEOWNERS",
            VcsProvider::GitLab => ".gitlab/CODEOWNERS",
            _ => "CODEOWNERS",
        });

        let mut generator = CodeownersGenerator {
            file: fs::create_file(&file_path)?,
            file_path,
            provider,
        };

        generator.write("# Automatically generated by moon. DO NOT MODIFY!")?;
        generator.write(format!("# Last generated: {}", Local::now()))?;
        generator.write("")?;

        Ok(generator)
    }

    pub fn add_project_entry(
        &mut self,
        name: &str,
        source: &str,
        config: &OwnersConfig,
    ) -> Result<(), FsError> {
        self.write("")?;

        // Render the header
        self.write(format!("# {}", name))?;

        match &self.provider {
            VcsProvider::Bitbucket => {
                if config.required_approvals > 1 {
                    if let Some(default_owner) = &config.default_owner {
                        self.write(format!(
                            "Check({} >= {})",
                            default_owner, config.required_approvals
                        ))?;
                    }
                }
            }

            VcsProvider::GitLab => {
                let mut header = format!("[{name}]");

                if config.optional {
                    header = format!("^{header}")
                }

                if config.required_approvals > 1 {
                    header = format!("{header}[{}]", config.required_approvals);
                }

                if matches!(config.paths, OwnersPaths::List(_)) {
                    header = format!("{header} {}", config.default_owner.as_ref().unwrap());
                }

                self.write(header)?;
            }
            _ => {}
        };

        // Render the owner entries
        let root = PathBuf::from("/").join(source);

        match &config.paths {
            OwnersPaths::List(paths) => {
                for path in paths {
                    if matches!(self.provider, VcsProvider::GitLab) {
                        self.write(self.format_path(root.join(path)))?;
                    } else {
                        self.write(format!(
                            "{} {}",
                            self.format_path(root.join(path)),
                            config.default_owner.as_ref().unwrap()
                        ))?;
                    }
                }
            }
            OwnersPaths::Map(map) => {
                for (path, owners) in map {
                    if owners.is_empty() {
                        self.write(format!(
                            "{} {}",
                            self.format_path(root.join(path)),
                            config.default_owner.as_ref().unwrap()
                        ))?;
                    } else {
                        self.write(format!(
                            "{} {}",
                            self.format_path(root.join(path)),
                            owners.join(" ")
                        ))?;
                    }
                }
            }
        };

        Ok(())
    }

    pub fn add_workspace_entries(&mut self, config: &CodeownersConfig) -> Result<(), FsError> {
        if config.global_paths.is_empty() {
            return Ok(());
        }

        self.write("")?;
        self.write("# (workspace)")?;

        for (path, owners) in &config.global_paths {
            if !owners.is_empty() {
                self.write(format!(
                    "{} {}",
                    self.format_path(PathBuf::from(path)),
                    owners.join(" ")
                ))?;
            }
        }

        Ok(())
    }

    pub fn generate(mut self) -> Result<(), FsError> {
        self.write("")?;

        self.file.flush().map_err(|error| FsError::Create {
            path: self.file_path.to_path_buf(),
            error,
        })?;

        Ok(())
    }

    fn format_path(&self, path: PathBuf) -> String {
        let path = path.to_string_lossy();

        if path.contains(' ') {
            return path.replace(" ", "\\ ");
        }

        path.to_string()
    }

    fn write<T: AsRef<str>>(&mut self, message: T) -> Result<(), FsError> {
        writeln!(self.file, "{}", message.as_ref()).map_err(|error| FsError::Create {
            path: self.file_path.to_path_buf(),
            error,
        })?;

        Ok(())
    }
}
