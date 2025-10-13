use moon_common::{Id, IdExt, color, path::WorkspaceRelativePath};
use moon_config::{
    ConfigLoader, DependencySource, InheritedTasksManager, InheritedTasksResult, LanguageType,
    ProjectConfig, ProjectDependencyConfig, ProjectDependsOn, TaskConfig, ToolchainConfig,
};
use moon_file_group::FileGroup;
use moon_project::Project;
use moon_task::Task;
use moon_task_builder::{TasksBuilder, TasksBuilderContext, create_project_dep_from_task_dep};
use moon_toolchain::filter_and_resolve_toolchain_ids;
use moon_toolchain_plugin::{ToolchainRegistry, api::DefineRequirementsInput};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{instrument, trace};

pub struct ProjectBuilderContext<'app> {
    pub config_loader: &'app ConfigLoader,
    pub enabled_toolchains: &'app [Id],
    pub monorepo: bool,
    pub root_project_id: Option<&'app Id>,
    pub toolchain_config: &'app ToolchainConfig,
    pub toolchain_registry: Arc<ToolchainRegistry>,
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
    aliases: Vec<String>,
    root: PathBuf,

    pub language: LanguageType,

    // Toolchains that will be used as the fallback for tasks.
    // These are filtered based on enabled.
    pub toolchains: Vec<Id>,

    // Toolchains that will be used for task/config inheritance.
    // These are *not* filtered based on enabled.
    pub toolchains_inheritance: Vec<Id>,

    // Inherited from the workspace-level enabled list,
    // but then also filtered further based on the project config
    enabled_toolchains: Vec<Id>,
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
            enabled_toolchains: context.enabled_toolchains.to_vec(),
            context,
            id,
            source,
            aliases: vec![],
            global_config: None,
            local_config: None,
            language: LanguageType::Unknown,
            toolchains: vec![],
            toolchains_inheritance: vec![],
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
            &self.toolchains_inheritance,
            &local_config.stack,
            &local_config.layer,
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
        let mut infer_toolchain_from_language = true;

        // Use configured language or detect from environment
        self.language = if config.language.is_unknown() {
            let language = self
                .context
                .toolchain_registry
                .detect_project_language(&self.root)
                .await?;

            trace!(
                project_id = self.id.as_str(),
                language = ?language,
                "Unknown project language, attempted to detect from environment",
            );

            infer_toolchain_from_language = false;
            language
        } else {
            config.language.clone()
        };

        // Determine toolchains that this project belongs to
        let mut toolchains = FxHashSet::default();

        // 1 - Explicitly configured by the user
        if let Some(default_ids) = &config.toolchains.default {
            toolchains.extend(default_ids.to_owned_list());
        }

        // 2 - Inferred from the language
        if infer_toolchain_from_language && !self.language.is_unknown() {
            toolchains.extend(
                self.context
                    .toolchain_registry
                    .detect_project_toolchain_from_language(&self.language)
                    .await?,
            );
        }

        // 2 - Detected from plugins
        toolchains.extend(
            self.context
                .toolchain_registry
                .detect_project_toolchain_from_usage(&self.root, |registry, toolchain| {
                    DefineRequirementsInput {
                        context: registry.create_context(),
                        toolchain_config: registry
                            .create_config(&toolchain.id, self.context.toolchain_config),
                    }
                })
                .await?,
        );

        // Filter down the toolchains list based on the project config
        for (plugin_id, override_config) in &config.toolchains.plugins {
            if override_config.is_enabled() {
                toolchains.insert(plugin_id.to_owned());
            } else {
                toolchains.remove(plugin_id);
            }
        }

        // Task inheritance relies entirely on stable IDs as the file
        // names are in the format of `tasks/node.yml`, etc
        self.toolchains_inheritance =
            Vec::from_iter(toolchains.iter().map(Id::stable).collect::<FxHashSet<_>>());

        // While the toolchains within a task use their literal
        // stable or unstable IDs based on what's configured/enabled
        self.toolchains = filter_and_resolve_toolchain_ids(
            &self.enabled_toolchains,
            toolchains.into_iter().collect(),
            true,
        );

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
    pub fn extend_with_dependency(&mut self, mut config: ProjectDependencyConfig) -> &mut Self {
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

    pub fn set_aliases(&mut self, aliases: Vec<String>) -> &mut Self {
        self.aliases = aliases.to_owned();
        self
    }

    #[instrument(name = "build_project", skip_all)]
    pub async fn build(mut self) -> miette::Result<Project> {
        // Build dependencies first since they're required for tasks
        let dependencies = self.build_dependencies()?;

        // Then build the tasks
        let tasks = self.build_tasks(&dependencies).await?;
        let task_targets = tasks
            .values()
            .map(|task| task.target.clone())
            .collect::<Vec<_>>();

        // And finally build the project
        let mut project = Project {
            dependencies,
            file_groups: self.build_file_groups()?,
            aliases: self.aliases,
            id: self.id.to_owned(),
            language: self.language,
            root: self.root,
            source: self.source.to_owned(),
            tasks,
            task_targets,
            toolchains: self.toolchains,
            ..Project::default()
        };

        project.inherited = self.global_config.take();

        let config = self.local_config.take().unwrap_or_default();

        project.stack = config.stack;
        project.layer = config.layer;
        project.config = config;
        project.toolchains.sort();

        resolve_project_dependencies(&mut project, self.context.root_project_id);

        Ok(project)
    }

    #[instrument(skip_all)]
    fn build_dependencies(&self) -> miette::Result<Vec<ProjectDependencyConfig>> {
        let mut deps = FxHashMap::default();

        trace!(
            project_id = self.id.as_str(),
            "Building project dependencies"
        );

        if let Some(local) = &self.local_config {
            for dep_on in &local.depends_on {
                let dep_config = match dep_on {
                    ProjectDependsOn::String(id) => ProjectDependencyConfig {
                        id: id.to_owned(),
                        ..Default::default()
                    },
                    ProjectDependsOn::Object(config) => config.to_owned(),
                };

                deps.insert(dep_config.id.clone(), dep_config);
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
    async fn build_tasks(
        &mut self,
        dependencies: &[ProjectDependencyConfig],
    ) -> miette::Result<BTreeMap<Id, Task>> {
        trace!(project_id = self.id.as_str(), "Building project tasks");

        let mut tasks_builder = TasksBuilder::new(
            self.id,
            dependencies,
            self.source,
            &self.toolchains,
            TasksBuilderContext {
                enabled_toolchains: &self.enabled_toolchains,
                monorepo: self.context.monorepo,
                toolchain_config: self.context.toolchain_config,
                toolchain_registry: self.context.toolchain_registry.clone(),
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

fn resolve_project_dependencies(project: &mut Project, root_project_id: Option<&Id>) {
    let mut deps: Vec<ProjectDependencyConfig> = vec![];

    // Tasks can depend on arbitrary projects, so include them also
    for task_config in project.tasks.values() {
        for task_dep in &task_config.deps {
            if let Some(dep_config) = create_project_dep_from_task_dep(
                task_dep,
                &project.id,
                root_project_id,
                |dep_project_id| {
                    deps.iter().any(|dep| &dep.id == dep_project_id)
                        || project
                            .dependencies
                            .iter()
                            .any(|dep| &dep.id == dep_project_id)
                        || project
                            .aliases
                            .iter()
                            .any(|alias| alias.as_str() == dep_project_id.as_str())
                },
            ) {
                deps.push(dep_config);
            }
        }
    }

    project.dependencies.extend(deps);
}
