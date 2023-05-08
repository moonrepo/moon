use moon_common::Id;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use starbase_utils::glob;
use std::path::PathBuf;
use tracing::debug;

use crate::FileGroupError;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct FileGroup {
    pub files: Vec<String>,

    pub globs: Vec<String>,

    pub id: Id,

    #[serde(skip)]
    walk_cache: OnceCell<Vec<PathBuf>>,
}

impl FileGroup {
    pub fn new<T, I, V>(id: T, patterns: I) -> Result<FileGroup, FileGroupError>
    where
        T: AsRef<str>,
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let id = id.as_ref();

        debug!(id, "Creating file group");

        let mut group = FileGroup {
            files: vec![],
            globs: vec![],
            id: Id::new(id)?,
            walk_cache: OnceCell::new(),
        };

        group.merge(patterns);

        Ok(group)
    }

    pub fn merge<I, V>(&mut self, patterns: I)
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        // Local files should always override global
        self.files = vec![];
        self.globs = vec![];

        for pattern in patterns {
            let pattern = pattern.as_ref();

            if glob::is_glob(pattern) {
                self.globs.push(pattern.to_owned());
            } else {
                self.files.push(pattern.to_owned());
            }
        }

        debug!(
            id = %self.id,
            files = self.files.join(", "),
            globs = self.globs.join(", "),
            "Updating file group with patterns",
        );
    }
}
