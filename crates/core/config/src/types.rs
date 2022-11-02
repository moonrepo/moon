use crate::TaskConfig;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

pub type FileGlob = String;

pub type FilePath = String;

pub type FilePathOrGlob = String;

pub type FileGroups = FxHashMap<String, Vec<FilePathOrGlob>>;

pub type InputValue = String; // file path, glob, env var

pub type ProjectAlias = String;

pub type ProjectsSourcesMap = FxHashMap<ProjectID, String>;

pub type ProjectsAliasesMap = FxHashMap<ProjectAlias, ProjectID>;

pub type ProjectID = String;

pub type TaskID = String;

pub type TasksConfigsMap = BTreeMap<TaskID, TaskConfig>;

pub type TargetID = String; // project_id:task_id
