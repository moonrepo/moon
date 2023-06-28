use moon_common::path::RelativePathBuf;
use moon_config::TemplateFrontmatterConfig;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Eq, PartialEq)]
pub enum FileState {
    Create,
    Merge,
    Replace,
    Skip,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TemplateFile {
    /// Frontmatter extracted into a config.
    pub config: Option<TemplateFrontmatterConfig>,

    /// Rendered and frontmatter-free file content.
    pub content: String,

    /// Absolute path to destination.
    pub dest_path: PathBuf,

    /// Relative path from templates dir. Also acts as the Tera engine name.
    pub name: RelativePathBuf,

    /// Absolute path to source (in templates dir).
    pub source_path: PathBuf,

    /// File state and operation to commit.
    pub state: FileState,
}

impl TemplateFile {
    pub fn new(name: RelativePathBuf, source_path: PathBuf) -> Self {
        TemplateFile {
            config: None,
            content: String::new(),
            dest_path: PathBuf::new(),
            name,
            source_path,
            state: FileState::Create,
        }
    }

    pub fn is_mergeable<'l>(&self) -> Option<&'l str> {
        let mut ext = self.name.as_str();

        if let Some(cfg) = &self.config {
            if let Some(to) = &cfg.to {
                ext = to;
            }
        }

        if ext.ends_with(".json") {
            return Some("json");
        } else if ext.ends_with(".yaml") || ext.ends_with(".yml") {
            return Some("yaml");
        }

        None
    }

    pub fn is_forced(&self) -> bool {
        self.config.as_ref().is_some_and(|cfg| cfg.force)
    }

    pub fn is_skipped(&self) -> bool {
        self.config.as_ref().is_some_and(|cfg| cfg.skip)
    }

    pub fn set_content<T: AsRef<str>>(&mut self, content: T, dest: &Path) -> miette::Result<()> {
        let content = content.as_ref().trim_start();

        self.dest_path = self.name.to_path(dest);

        if content.starts_with("---") {
            debug!(
                file = %self.name,
                "Found frontmatter in template file, extracting",
            );

            if let Some(fm_end) = &content[4..].find("---") {
                let end_index = fm_end + 4;
                let config = TemplateFrontmatterConfig::parse(&content[4..end_index])?;

                if let Some(to) = &config.to {
                    self.dest_path = dest.join(to);
                }

                self.config = Some(config);
                self.content = content[(end_index + 4)..].trim_start().to_owned();

                return Ok(());
            }
        }

        self.content = content.to_owned();

        Ok(())
    }

    pub fn should_write(&self) -> bool {
        !matches!(self.state, FileState::Skip)
    }
}
