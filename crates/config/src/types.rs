use std::collections::HashMap;

pub type FilePath = String;

pub type FilePathOrGlob = String;

pub type FileGroups = HashMap<String, Vec<FilePathOrGlob>>;

pub type ProjectID = String;

pub type TaskID = String;

pub type TargetID = String; // project_id:task_id
