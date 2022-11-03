use crate::TaskConfig;
use std::collections::{BTreeMap, HashMap};

pub type FileGlob = String;

pub type FilePath = String;

pub type FilePathOrGlob = String;

pub type FileGroups = HashMap<String, Vec<FilePathOrGlob>>;

pub type InputValue = String; // file path, glob, env var

pub type ProjectAlias = String;

pub type ProjectsSourcesMap = HashMap<ProjectID, String>;

pub type ProjectsAliasesMap = HashMap<ProjectAlias, ProjectID>;

pub type ProjectID = String;

pub type TaskID = String;

pub type TasksConfigsMap = BTreeMap<TaskID, TaskConfig>;

pub type TargetID = String; // project_id:task_id
