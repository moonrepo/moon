use crate::project::TaskConfig;
use crate::shapes::InputPath;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

pub type FileGroupsMap = FxHashMap<Id, Vec<InputPath>>;

pub type ProjectSourceEntry = (Id, WorkspaceRelativePathBuf);

pub type ProjectsSourcesList = Vec<ProjectSourceEntry>;

pub type ProjectAliasEntry = (Id, String);

pub type ProjectsAliasesList = Vec<ProjectAliasEntry>;

pub type TasksConfigsMap = BTreeMap<Id, TaskConfig>;

#[cfg(feature = "target")]
pub use moon_target::Target;

#[cfg(not(feature = "target"))]
pub type Target = String;
