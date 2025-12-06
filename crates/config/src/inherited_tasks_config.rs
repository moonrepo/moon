use crate::config_struct;
use crate::patterns::{merge_iter, merge_tasks_partials};
use crate::project::LanguageType;
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

#[derive(Default)]
pub struct InheritFor<'a> {
    pub language: Option<&'a LanguageType>,
    pub layer: Option<&'a LayerType>,
    pub root: Option<&'a Path>,
    pub stack: Option<&'a StackType>,
    pub tags: Option<&'a [Id]>,
    pub toolchains: Option<&'a [Id]>,
}

impl<'a> InheritFor<'a> {
    pub fn language(mut self, language: &'a LanguageType) -> Self {
        self.language = Some(language);
        self
    }

    pub fn layer(mut self, layer: &'a LayerType) -> Self {
        self.layer = Some(layer);
        self
    }

    pub fn root(mut self, root: &'a Path) -> Self {
        self.root = Some(root);
        self
    }

    pub fn stack(mut self, stack: &'a StackType) -> Self {
        self.stack = Some(stack);
        self
    }

    pub fn tags(mut self, tags: &'a [Id]) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn toolchains(mut self, toolchains: &'a [Id]) -> Self {
        self.toolchains = Some(toolchains);
        self
    }
}

config_struct!(
    /// A condition that utilizes a combination of logical operators
    /// to match against. When matching, all clauses must be satisfied.
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

impl PartialInheritedClauseConfig {
    pub fn matches(&self, values: &[Id]) -> bool {
        if values.is_empty() || self.not.is_none() && self.and.is_none() && self.or.is_none() {
            return false;
        }

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

        true
    }
}

config_struct!(
    /// Patterns in which a condition can be configured as.
    #[derive(Config)]
    #[serde(untagged)]
    pub enum InheritedConditionConfig {
        /// Condition applies to a single value.
        One(Id),

        /// Condition applies to multiple values,
        /// and matches using an OR operator.
        Many(Vec<Id>),

        /// Condition applies using logical operator clauses.
        #[setting(nested)]
        Clause(InheritedClauseConfig),
    }
);

impl PartialInheritedConditionConfig {
    pub fn matches(&self, values: &[Id]) -> bool {
        match self {
            Self::Clause(inner) => inner.matches(values),
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
        /// The order in which this configuration is inherited by a project.
        /// Lower is inherited first, while higher is last.
        pub order: Option<u16>,

        /// Condition that matches against literal files within a project.
        /// If multiple values are provided, at least 1 file needs to exist.
        #[setting(alias = "file")]
        pub files: Option<OneOrMany<FilePath>>,

        /// Condition that matches against a project's `language`.
        /// If multiple values are provided, it matches using an OR operator.
        #[setting(alias = "language")]
        pub languages: Option<OneOrMany<LanguageType>>,

        /// Condition that matches against a project's `layer`.
        /// If multiple values are provided, it matches using an OR operator.
        #[setting(alias = "layer")]
        pub layers: Option<OneOrMany<LayerType>>,

        /// Condition that matches against a project's `stack`.
        /// If multiple values are provided, it matches using an OR operator.
        #[setting(alias = "stack")]
        pub stacks: Option<OneOrMany<StackType>>,

        /// Condition that matches against a tag within the project.
        #[setting(alias = "tag", nested)]
        pub tags: Option<InheritedConditionConfig>,

        /// Condition that matches against a toolchain detected for a project.
        #[setting(alias = "toolchain", nested)]
        pub toolchains: Option<InheritedConditionConfig>,
    }
);

impl PartialInheritedByConfig {
    pub fn default_toolchain(&self) -> Option<Id> {
        self.toolchains.as_ref().and_then(|entry| match entry {
            PartialInheritedConditionConfig::One(id) => Some(id.to_owned()),
            PartialInheritedConditionConfig::Many(ids) => {
                if ids.len() == 1 {
                    Some(ids[0].to_owned())
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    // 0 - (files)
    // 50 - node
    // 100 - frontend
    // 150 - library
    // 150 - node-frontend
    // 200 - node-library
    // 250 - frontend-library
    // 300 - node-frontend-library
    // 500 - (tags)
    pub fn order(&self) -> u16 {
        if let Some(order) = self.order {
            return order;
        }

        let mut amount = 0;

        // Toolchains/languages are the lowest level
        if self.toolchains.is_some() || self.languages.is_some() {
            amount += 50;
        }

        // Stacks are the middle level
        if self.stacks.is_some() {
            amount += 100;
        }

        // Layers are the highest level
        if self.layers.is_some() {
            amount += 150;
        }

        // Tags are their own level (typically)
        if self.tags.is_some() {
            amount += 500;
        }

        amount
    }

    pub fn matches(&self, input: &InheritFor) -> bool {
        if let Some(condition) = &self.stacks
            && let Some(value) = &input.stack
            && !condition.matches(value)
        {
            return false;
        }

        if let Some(condition) = &self.languages
            && let Some(value) = &input.language
            && !condition.matches(value)
        {
            return false;
        }

        if let Some(condition) = &self.layers
            && let Some(value) = &input.layer
            && !condition.matches(value)
        {
            return false;
        }

        if let Some(condition) = &self.tags
            && let Some(value) = &input.tags
            && !condition.matches(value)
        {
            return false;
        }

        if let Some(condition) = &self.toolchains
            && let Some(value) = &input.toolchains
            && !condition.matches(value)
        {
            return false;
        }

        if let Some(files) = &self.files
            && let Some(value) = &input.root
            && !files.to_list().iter().any(|file| value.join(file).exists())
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
        /// @since 2.0.0
        #[setting(nested)]
        pub inherited_by: Option<InheritedByConfig>,

        /// A map of identifiers to task objects. Tasks represent the work-unit
        /// of a project, and can be ran in the action pipeline.
        #[setting(nested, merge = merge_tasks_partials)]
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
        #[deprecated]
        pub config: InheritedTasksConfig,
        pub configs: IndexMap<String, InheritedTasksConfig>,
        #[deprecated]
        pub layers: IndexMap<String, PartialInheritedTasksConfig>,
        pub task_layers: FxHashMap<String, Vec<String>>,
    }
);
