use crate::task_options::TaskOptions;
use moon_common::{cacheable, path::WorkspaceRelativePathBuf, Id};
use moon_config::{InputPath, OutputPath, PlatformType, TaskType};
use moon_target::Target;
use rustc_hash::{FxHashMap, FxHashSet};

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct Task {
        pub args: Vec<String>,

        pub command: String,

        pub deps: Vec<Target>,

        pub env: FxHashMap<String, String>,

        pub id: Id,

        pub inputs: Vec<InputPath>,

        pub input_globs: FxHashSet<WorkspaceRelativePathBuf>,

        pub input_paths: FxHashSet<WorkspaceRelativePathBuf>,

        pub input_vars: FxHashSet<String>,

        pub options: TaskOptions,

        pub outputs: Vec<OutputPath>,

        pub output_globs: FxHashSet<WorkspaceRelativePathBuf>,

        pub output_paths: FxHashSet<WorkspaceRelativePathBuf>,

        pub platform: PlatformType,

        pub target: Target,

        #[serde(rename = "type")]
        pub type_of: TaskType,
    }
);
