use crate::project::TaskConfig;
use crate::shapes::InputPath;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

pub type InputGroupsMap = FxHashMap<Id, Vec<InputPath>>;

pub type ProjectSourceEntry = (Id, WorkspaceRelativePathBuf);

pub type ProjectsSourcesList = Vec<ProjectSourceEntry>;

pub type ProjectAliasEntry = (Id, String);

pub type ProjectsAliasesList = Vec<ProjectAliasEntry>;

pub type TasksConfigsMap = BTreeMap<Id, TaskConfig>;
