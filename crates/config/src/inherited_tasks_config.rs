use crate::config_struct;
use crate::patterns::merge_iter;
use crate::project_config::{LayerType, StackType};
use crate::shapes::{FilePath, Input, OneOrMany};
use crate::task_config::{TaskConfig, TaskDependency, validate_deps};
use crate::task_options_config::{PartialTaskOptionsConfig, TaskOptionsConfig};
use moon_common::{Id, cacheable};
use rustc_hash::FxHashMap;
use schematic::schema::indexmap::IndexMap;
use schematic::{Config, merge, validate};
use std::collections::BTreeMap;
use std::path::Path;

config_struct!(
    /// A condition that utilizes a combination of logical operators
    /// to match against.
    #[derive(Config)]
    pub struct InheritedClauseConfig {
        /// Require all values to match, using an AND operator.
        pub and: Option<OneOrMany<Id>>,

        /// Require any values to match, using an OR operator.
        pub or: Option<OneOrMany<Id>>,

        /// Require no values to match, using a NOT operator.
        pub not: Option<OneOrMany<Id>>,
    }
);

impl InheritedClauseConfig {
    pub fn matches(&self, values: &[Id]) -> bool {
        if let Some(not) = &self.not
            && not.to_list().iter().any(|value| values.contains(value))
        {
            return false;
        }

        if let Some(and) = &self.and
            && !and.to_list().iter().all(|value| values.contains(value))
        {
            return false;
        }

        if let Some(or) = &self.or
            && !or.to_list().iter().any(|value| values.contains(value))
        {
            return false;
        }

        if self.not.is_none() && self.and.is_none() && self.or.is_none() {
            return false;
        }

        true
    }
}

config_struct!(
    /// Patterns in which a condition can be configured as.
    #[derive(Config)]
    #[serde(untagged)]
    pub enum InheritedConditionConfig {
        /// Condition applies using logical operator clauses.
        #[setting(nested)]
        Clause(InheritedClauseConfig),

        /// Condition applies to multiple values,
        /// and matches using an OR operator.
        Many(Vec<Id>),

        /// Condition applies to a single value.
        One(Id),
    }
);

impl InheritedConditionConfig {
    pub fn matches(&self, values: &[Id]) -> bool {
        match self {
            Self::Clause(inner) => inner.matches(values),
            // OR match
            Self::Many(inner) => values.iter().any(|value| inner.contains(value)),
            Self::One(inner) => values.contains(inner),
        }
    }
}

config_struct!(
    /// Configures conditions that must match against a project for tasks
    /// to be inherited. If multiple conditions are defined, then all must match
    /// for inheritance to occur. If no conditions are defined, then tasks will
    /// be inherited by all projects.
    #[derive(Config)]
    pub struct InheritedByConfig {
        /// Condition that matches against literal files within a project.
        /// If multiple values are provided, at least 1 file needs to exist.
        pub files: Option<OneOrMany<FilePath>>,

        /// Condition that matches against a project's `layer`.
        /// If multiple values are provided, it matches using an OR operator.
        pub layers: Option<OneOrMany<LayerType>>,

        /// Condition that matches against a project's `stack`.
        /// If multiple values are provided, it matches using an OR operator.
        pub stacks: Option<OneOrMany<StackType>>,

        /// Condition that matches against a tag within the project.
        pub tags: Option<InheritedConditionConfig>,

        /// Condition that matches against a toolchain detected for a project.
        pub toolchains: Option<InheritedConditionConfig>,
    }
);

impl InheritedByConfig {
    pub fn matches(
        &self,
        root: &Path,
        stack: &StackType,
        layer: &LayerType,
        toolchains: &[Id],
        tags: &[Id],
    ) -> bool {
        if let Some(condition) = &self.stacks
            && *stack != StackType::Unknown
            && !condition.matches(stack)
        {
            return false;
        }

        if let Some(condition) = &self.layers
            && *layer != LayerType::Unknown
            && !condition.matches(layer)
        {
            return false;
        }

        if let Some(condition) = &self.tags
            && !tags.is_empty()
            && !condition.matches(tags)
        {
            return false;
        }

        if let Some(condition) = &self.toolchains
            && !toolchains.is_empty()
            && !condition.matches(toolchains)
        {
            return false;
        }

        if let Some(files) = &self.files
            && !files.to_list().iter().any(|file| root.join(file).exists())
        {
            return false;
        }

        true
    }
}

config_struct!(
    /// Configures tasks and task related settings that'll be inherited by all
    /// matching projects.
    /// Docs: https://moonrepo.dev/docs/config/tasks
    #[derive(Config)]
    pub struct InheritedTasksConfig {
        #[setting(default = "./cache/schemas/tasks.json", rename = "$schema")]
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

        /// A map of conditions that define which projects will inherit these
        /// tasks and configuration. If not defined, will be inherited by all projects.
        pub inherited_by: Option<InheritedByConfig>,

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
