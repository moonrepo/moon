use crate::config_struct;
use crate::patterns::merge_iter;
use crate::shapes::Input;
use crate::task_config::{TaskConfig, TaskDependency, validate_deps};
use crate::task_options_config::{PartialTaskOptionsConfig, TaskOptionsConfig};
use moon_common::{Id, cacheable};
use rustc_hash::FxHashMap;
use schematic::schema::indexmap::IndexMap;
use schematic::{Config, merge, validate};
use std::collections::BTreeMap;

config_struct!(
    /// Configures tasks and task related settings that'll be inherited by all
    /// matching projects.
    /// Docs: https://moonrepo.dev/docs/config/tasks
    #[derive(Config)]
    pub struct InheritedTasksConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/tasks.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Extends one or many tasks configuration files.
        /// Supports a relative file path or a secure URL.
        /// @since 1.12.0
        #[setting(extend, validate = validate::extends_from)]
        pub extends: Option<schematic::ExtendsFrom>,

        /// A map of group identifiers to a list of file paths, globs, and
        /// environment variables, that can be referenced from tasks.
        #[setting(merge = merge_iter)]
        pub file_groups: FxHashMap<Id, Vec<Input>>,

        /// Task dependencies (`deps`) that will be automatically injected into every
        /// task that inherits this configuration.
        #[setting(nested, merge = merge::append_vec, validate = validate_deps)]
        pub implicit_deps: Vec<TaskDependency>,

        /// Task inputs (`inputs`) that will be automatically injected into every
        /// task that inherits this configuration.
        #[setting(merge = merge::append_vec)]
        pub implicit_inputs: Vec<Input>,

        /// A map of identifiers to task objects. Tasks represent the work-unit
        /// of a project, and can be ran in the action pipeline.
        #[setting(nested, merge = merge::merge_btreemap)]
        pub tasks: BTreeMap<Id, TaskConfig>,

        /// Default task options for all inherited tasks.
        /// @since 1.20.0
        #[setting(nested)]
        pub task_options: Option<TaskOptionsConfig>,
    }
);

cacheable!(
    #[derive(Clone, Debug, Default)]
    pub struct InheritedTasksResult {
        pub order: Vec<String>,
        pub config: InheritedTasksConfig,
        pub layers: IndexMap<String, PartialInheritedTasksConfig>,
        pub task_layers: FxHashMap<String, Vec<String>>,
    }
);
