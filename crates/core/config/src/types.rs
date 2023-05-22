use crate::TaskConfig;
use moon_common::Id;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

pub type FileGlob = String;

pub type FilePath = String;

pub type FilePathOrGlob = String;

pub type FileGroups = FxHashMap<Id, Vec<FilePathOrGlob>>;

pub type InputValue = String; // file path, glob, env var

pub type ProjectAlias = String;

pub type ProjectsSourcesMap = FxHashMap<Id, String>;

pub type ProjectsAliasesMap = FxHashMap<ProjectAlias, Id>;

pub type TasksConfigsMap = BTreeMap<Id, TaskConfig>;
