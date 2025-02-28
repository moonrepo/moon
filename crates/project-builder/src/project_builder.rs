use moon_common::path::WorkspaceRelativePath;
use moon_common::{Id, color};
use moon_config::{
    ConfigLoader, DependencyConfig, DependencyScope, DependencySource, InheritedTasksManager,
    InheritedTasksResult, LanguageType, ProjectConfig, ProjectDependsOn, TaskConfig,
    ToolchainConfig,
};
use moon_file_group::FileGroup;
use moon_project::Project;
use moon_task::{TargetScope, Task};
use moon_task_builder::{TasksBuilder, TasksBuilderContext};
use moon_toolchain::detect::{
    detect_project_language, detect_project_toolchains, get_project_toolchains,
};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, trace};

pub struct ProjectBuilderContext<'app> {
    pub config_loader: &'app ConfigLoader,
    pub enabled_toolchains: &'app [Id],
    pub monorepo: bool,
    pub root_project_id: Option<&'app Id>,
    pub toolchain_config: &'app ToolchainConfig,
    pub workspace_root: &'app Path,
}

pub struct ProjectBuilder<'app> {
    context: ProjectBuilderContext<'app>,

    // Configs to derive information from
    global_config: Option<InheritedTasksResult>,
    local_config: Option<ProjectConfig>,

    // Values to be continually built
    id: &'app Id,
    source: &'app WorkspaceRelativePath,
    alias: Option<&'app str>,
    root: PathBuf,

    pub language: LanguageType,

    // Toolchains that will be used as the fallback for tasks.
    // These are filtered based on enabled.
    pub toolchains_tasks: Vec<Id>,

    // Toolchains that will be used for task/config inheritance.
    // These are *not* filtered based on enabled.
    pub toolchains_config: Vec<Id>,
}

impl<'app> ProjectBuilder<'app> {
    pub fn new(
        id: &'app Id,
        source: &'app WorkspaceRelativePath,
        context: ProjectBuilderContext<'app>,
    ) -> miette::Result<Self> {
        trace!(
            project_id = id.as_str(),
            source = source.as_str(),
            "Building project {} from source",
            color::id(id)
        );

        Ok(ProjectBuilder {
            root: source.to_logical_path(context.workspace_root),
            context,
            id,
            source,
            alias: None,
            global_config: None,
            local_config: None,
            language: LanguageType::Unknown,
            toolchains_tasks: vec![],
            toolchains_config: vec![],
        })
    }

    /// Inherit tasks, file groups, and more from global `.moon/tasks` configs.
    #[instrument(skip_all)]
    pub fn inherit_global_config(
        &mut self,
        tasks_manager: &InheritedTasksManager,
    ) -> miette::Result<&mut Self> {
        let local_config = self
            .local_config
            .as_ref()
            .expect("Local config must be loaded before global config!");

        let global_config = tasks_manager.get_inherited_config(
            &self.toolchains_config,
            &local_config.stack,
            &local_config.type_of,
            &local_config.tags,
        )?;

        trace!(
            project_id = self.id.as_str(),
            lookup = ?global_config.order,
            "Inheriting global file groups and tasks",
        );

        self.global_config = Some(global_config);

        Ok(self)
    }

    /// Inherit the local config and then detect applicable language and toolchain fields.
    #[instrument(skip_all)]
    pub async fn inherit_local_config(&mut self, config: &ProjectConfig) -> miette::Result<()> {
        // Use configured language or detect from environment
        self.language = if config.language == LanguageType::Unknown {
            let language = detect_project_language(&self.root);

            trace!(
                project_id = self.id.as_str(),
                language = ?language,
                "Unknown project language, detecting from environment",
            );

            language
        } else {
            config.language.clone()
        };

        // Infer toolchains from the language as it handles the chain correctly:
        // For example: node -> javascript, and not just node
        if self.toolchains_tasks.is_empty() {
            let mut toolchains = vec![];

            #[allow(deprecated)]
            if let Some(default_id) = &config.toolchain.default {
                toolchains.extend(get_project_toolchains(default_id, &self.language));
            } else if let Some(platform) = &config.platform {
                let default_id = platform.get_toolchain_id();

                toolchains.extend(get_project_toolchains(&default_id, &self.language));

                debug!(
                    project_id = self.id.as_str(),
                    "The {} project setting has been deprecated, use {} instead, or rely on configuration/environment detection instead",
                    color::property("platform"),
                    color::property("toolchain.default"),
                );
            } else {
                toolchains.extend(detect_project_toolchains(
                    self.context.workspace_root,
                    &self.root,
                    &self.language,
                ));
            }

            self.toolchains_config.extend(toolchains.clone());

            self.toolchains_tasks = toolchains
                .into_iter()
                .filter(|id| self.context.enabled_toolchains.contains(id))
                .collect();

            trace!(
                project_id = self.id.as_str(),
                language = ?self.language,
                toolchains = ?self.toolchains_tasks.iter().map(|tc| tc.as_str()).collect::<Vec<_>>(),
                "Unknown tasks toolchain, inferring from project language",
            );
        }

        self.local_config = Some(config.to_owned());

        Ok(())
    }

    /// Load a `moon.*` config file from the root of the project (derived from source).
    #[instrument(skip_all)]
    pub async fn load_local_config(&mut self) -> miette::Result<()> {
        let config = self.context.config_loader.load_project_config(&self.root)?;

        self.inherit_local_config(&config).await?;

        Ok(())
    }

    /// Extend the builder with a project dependency implicitly derived from the project graph.
    /// Implicit dependencies *must not* override explicitly configured dependencies.
    pub fn extend_with_dependency(&mut self, mut config: DependencyConfig) -> &mut Self {
        let local_config = self
            .local_config
            .as_mut()
            .expect("Local config must be loaded before extending dependencies!");

        config.source = DependencySource::Implicit;

        local_config
            .depends_on
            .push(ProjectDependsOn::Object(config));

        self
    }

    /// Extend the builder with a toolchain specific task implicitly derived from the project graph.
    /// Implicit tasks *must not* override explicitly configured tasks.
    pub fn extend_with_task(&mut self, id: Id, config: TaskConfig) -> &mut Self {
        let local_config = self
            .local_config
            .as_mut()
            .expect("Local config must be loaded before extending tasks!");

        local_config.tasks.entry(id).or_insert(config);

        self
    }

    pub fn set_alias(&mut self, alias: &'app str) -> &mut Self {
        self.alias = Some(alias);
        self
    }

    #[instrument(name = "build_project", skip_all)]
    pub async fn build(mut self) -> miette::Result<Project> {
        let tasks = self.build_tasks().await?;
        let task_targets = tasks
            .values()
            .map(|task| task.target.clone())
            .collect::<Vec<_>>();

        let mut project = Project {
            alias: self.alias.map(|a| a.to_owned()),
            dependencies: self.build_dependencies(&tasks)?,
            file_groups: self.build_file_groups()?,
            task_targets,
            tasks,
            id: self.id.to_owned(),
            language: self.language,
            root: self.root,
            source: self.source.to_owned(),
            // Should this be the config one?
            toolchains: if self.toolchains_tasks.is_empty() {
                vec![Id::raw("system")]
            } else {
                self.toolchains_tasks
            },
            ..Project::default()
        };

        project.inherited = self.global_config.take();

        let config = self.local_config.take().unwrap_or_default();

        project.stack = config.stack;
        project.type_of = config.type_of;
        project.config = config;

        Ok(project)
    }

    #[instrument(skip_all)]
    fn build_dependencies(
        &self,
        tasks: &BTreeMap<Id, Task>,
    ) -> miette::Result<Vec<DependencyConfig>> {
        let mut deps = FxHashMap::default();

        trace!(
            project_id = self.id.as_str(),
            "Building project dependencies"
        );

        if let Some(local) = &self.local_config {
            for dep_on in &local.depends_on {
                let dep_config = match dep_on {
                    ProjectDependsOn::String(id) => DependencyConfig {
                        id: id.to_owned(),
                        ..DependencyConfig::default()
                    },
                    ProjectDependsOn::Object(config) => config.to_owned(),
                };

                deps.insert(dep_config.id.clone(), dep_config);
            }
        }

        // Tasks can depend on arbitrary projects, so include them also
        for task_config in tasks.values() {
            for task_dep in &task_config.deps {
                if let TargetScope::Project(dep_id) = &task_dep.target.scope {
                    // Already a dependency, or references self
                    if deps.contains_key(dep_id)
                        || self.id == dep_id
                        || self.alias.as_ref().is_some_and(|a| *a == dep_id.as_str())
                    {
                        continue;
                    }

                    trace!(
                        project_id = self.id.as_str(),
                        dep_id = dep_id.as_str(),
                        task_target = task_config.target.as_str(),
                        "Marking arbitrary project as an implicit dependency because of a task dependency"
                    );

                    deps.insert(
                        dep_id.to_owned(),
                        DependencyConfig {
                            id: dep_id.to_owned(),
                            scope: if self.context.root_project_id.is_some_and(|id| id == dep_id) {
                                DependencyScope::Root
                            } else {
                                DependencyScope::Build
                            },
                            source: DependencySource::Implicit,
                            via: Some(task_config.target.to_string()),
                        },
                    );
                }
            }
        }

        if !deps.is_empty() {
            trace!(
                project_id = self.id.as_str(),
                dep_ids = ?deps.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                "Depends on {} projects",
                deps.len(),
            );
        }

        Ok(deps.into_values().collect::<Vec<_>>())
    }

    #[instrument(skip_all)]
    fn build_file_groups(&self) -> miette::Result<BTreeMap<Id, FileGroup>> {
        let mut file_inputs = BTreeMap::default();
        let project_source = &self.source;

        trace!(project_id = self.id.as_str(), "Building file groups");

        // Inherit global first
        if let Some(global) = &self.global_config {
            trace!(
                project_id = self.id.as_str(),
                groups = ?global.config.file_groups.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                "Inheriting global file groups",
            );

            for (id, inputs) in &global.config.file_groups {
                file_inputs.insert(id, inputs);
            }
        }

        // Override with local second
        if let Some(local) = &self.local_config {
            trace!(
                project_id = self.id.as_str(),
                groups = ?local.file_groups.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                "Using local file groups",
            );

            for (id, inputs) in &local.file_groups {
                file_inputs.insert(id, inputs);
            }
        }

        // And finally convert to a file group instance
        let mut file_groups = BTreeMap::default();

        for (id, inputs) in file_inputs {
            let mut group = FileGroup::new(id)?;
            group.add_many(inputs, project_source.as_str())?;

            file_groups.insert(id.to_owned(), group);
        }

        Ok(file_groups)
    }

    #[instrument(skip_all)]
    async fn build_tasks(&mut self) -> miette::Result<BTreeMap<Id, Task>> {
        trace!(project_id = self.id.as_str(), "Building tasks");

        let mut tasks_builder = TasksBuilder::new(
            self.id,
            self.source,
            &self.toolchains_tasks,
            TasksBuilderContext {
                enabled_toolchains: self.context.enabled_toolchains,
                monorepo: self.context.monorepo,
                toolchain_config: self.context.toolchain_config,
                workspace_root: self.context.workspace_root,
            },
        );

        if let Some(global_config) = &self.global_config {
            tasks_builder.inherit_global_tasks(
                &global_config.config,
                self.local_config
                    .as_ref()
                    .map(|cfg| &cfg.workspace.inherited_tasks),
            );
        }

        if let Some(local_config) = &self.local_config {
            tasks_builder.load_local_tasks(local_config);
        }

        tasks_builder.build().await
    }
}
