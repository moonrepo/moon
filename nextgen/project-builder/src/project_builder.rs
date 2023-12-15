use moon_common::path::WorkspaceRelativePath;
use moon_common::{color, consts, Id};
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, InheritedTasksManager,
    InheritedTasksResult, LanguageType, PlatformType, ProjectConfig, ProjectDependsOn, TaskConfig,
    ToolchainConfig,
};
use moon_file_group::FileGroup;
use moon_project::Project;
use moon_task::{TargetScope, Task};
use moon_task_builder::{DetectPlatformEvent, TasksBuilder, TasksBuilderContext};
use rustc_hash::FxHashMap;
use starbase_events::{Emitter, Event};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

#[derive(Debug)]
pub struct DetectLanguageEvent {
    pub project_root: PathBuf,
}

impl Event for DetectLanguageEvent {
    type Data = LanguageType;
}

pub struct ProjectBuilderContext<'app> {
    pub detect_language: &'app Emitter<DetectLanguageEvent>,
    pub detect_platform: &'app Emitter<DetectPlatformEvent>,
    pub legacy_task_inheritance: bool,
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
    id: Cow<'app, Id>,
    source: &'app WorkspaceRelativePath,
    alias: Option<&'app str>,
    project_root: PathBuf,

    pub language: LanguageType,
    pub platform: PlatformType,
}

impl<'app> ProjectBuilder<'app> {
    pub fn new(
        id: &'app Id,
        source: &'app WorkspaceRelativePath,
        context: ProjectBuilderContext<'app>,
    ) -> miette::Result<Self> {
        trace!(
            id = id.as_str(),
            source = source.as_str(),
            "Building project {} from source",
            color::id(id)
        );

        Ok(ProjectBuilder {
            project_root: source.to_logical_path(context.workspace_root),
            context,
            id: Cow::Borrowed(id),
            source,
            alias: None,
            global_config: None,
            local_config: None,
            language: LanguageType::Unknown,
            platform: PlatformType::Unknown,
        })
    }

    /// Inherit tasks, file groups, and more from global `.moon/tasks` configs.
    pub fn inherit_global_config(
        &mut self,
        tasks_manager: &InheritedTasksManager,
    ) -> miette::Result<&mut Self> {
        let local_config = self
            .local_config
            .as_ref()
            .expect("Local config must be loaded before global config!");

        let global_config = tasks_manager.get_inherited_config(
            &self.platform,
            &self.language,
            &local_config.type_of,
            &local_config.tags,
        )?;

        trace!(
            id = self.id.as_str(),
            lookup = ?global_config.order,
            "Inheriting global file groups and tasks",
        );

        self.global_config = Some(global_config);

        Ok(self)
    }

    /// Inherit the local config and then detect applicable language and platform fields.
    pub async fn inherit_local_config(&mut self, config: ProjectConfig) -> miette::Result<()> {
        // Use configured language or detect from environment
        self.language = if config.language == LanguageType::Unknown {
            let mut language = self
                .context
                .detect_language
                .emit(DetectLanguageEvent {
                    project_root: self.project_root.clone(),
                })
                .await?;

            if language == LanguageType::Unknown {
                language = config.language.clone();
            }

            trace!(
                id = self.id.as_str(),
                language = ?language,
                "Unknown project language, detecting from environment",
            );

            language
        } else {
            config.language.clone()
        };

        // Use configured platform or infer from language
        self.platform = config.platform.unwrap_or_else(|| {
            let platform: PlatformType = self.language.clone().into();

            trace!(
                id = self.id.as_str(),
                language = ?self.language,
                platform = ?self.platform,
                "Unknown tasks platform, inferring from language",
            );

            platform
        });

        // Inherit the custom ID
        if let Some(new_id) = &config.id {
            if new_id != self.id.as_ref() {
                debug!(
                    old_id = self.id.as_str(),
                    new_id = new_id.as_str(),
                    "Project has been configured with an explicit identifier of {}, renaming from {}",
                    color::id(new_id),
                    color::id(self.id.as_str()),
                );

                self.id = Cow::Owned(new_id.to_owned());
            }
        }

        self.local_config = Some(config);

        Ok(())
    }

    /// Load a `moon.yml` config file from the root of the project (derived from source).
    pub async fn load_local_config(&mut self) -> miette::Result<()> {
        let config_name = self.source.join(consts::CONFIG_PROJECT_FILENAME);
        let config_path = config_name.to_path(self.context.workspace_root);

        debug!(
            id = self.id.as_str(),
            file = ?config_path,
            "Attempting to load {} (optional)",
            color::file(config_name.as_str())
        );

        let config = ProjectConfig::load(self.context.workspace_root, config_path)?;

        self.inherit_local_config(config).await?;

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

    /// Extend the builder with a platform specific task implicitly derived from the project graph.
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

    #[tracing::instrument(name = "project", skip_all)]
    pub async fn build(mut self) -> miette::Result<Project> {
        let tasks = self.build_tasks().await?;

        let mut project = Project {
            alias: self.alias.map(|a| a.to_owned()),
            dependencies: self.build_dependencies(&tasks)?,
            file_groups: self.build_file_groups()?,
            tasks,
            id: self.id.into_owned(),
            language: self.language,
            platform: self.platform,
            root: self.project_root,
            source: self.source.to_owned(),
            ..Project::default()
        };

        project.inherited = self.global_config.take();

        let config = self.local_config.take().unwrap_or_default();

        project.type_of = config.type_of;
        project.config = config;

        Ok(project)
    }

    fn build_dependencies(
        &self,
        tasks: &BTreeMap<Id, Task>,
    ) -> miette::Result<Vec<DependencyConfig>> {
        let mut deps = FxHashMap::default();

        trace!(id = self.id.as_str(), "Building project dependencies");

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

        // Tasks can depend on arbitray projects, so include them also
        for task_config in tasks.values() {
            for task_dep in &task_config.deps {
                if let TargetScope::Project(dep_id) = &task_dep.scope {
                    // Already a dependency, or references self
                    if deps.contains_key(dep_id)
                        || self.id.as_ref() == dep_id
                        || self.alias.as_ref().is_some_and(|a| *a == dep_id.as_str())
                    {
                        continue;
                    }

                    trace!(
                        id = self.id.as_str(),
                        dep = dep_id.as_str(),
                        task = task_config.target.as_str(),
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
                id = self.id.as_str(),
                deps = ?deps.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                "Depends on {} projects",
                deps.len(),
            );
        }

        Ok(deps.into_values().collect::<Vec<_>>())
    }

    fn build_file_groups(&self) -> miette::Result<FxHashMap<Id, FileGroup>> {
        let mut file_inputs = FxHashMap::default();
        let project_source = &self.source;

        trace!(id = self.id.as_str(), "Building file groups");

        // Inherit global first
        if let Some(global) = &self.global_config {
            trace!(
                id = self.id.as_str(),
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
                id = self.id.as_str(),
                groups = ?local.file_groups.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                "Using local file groups",
            );

            for (id, inputs) in &local.file_groups {
                file_inputs.insert(id, inputs);
            }
        }

        // And finally convert to a file group instance
        let mut file_groups = FxHashMap::default();

        for (id, inputs) in file_inputs {
            file_groups.insert(
                id.to_owned(),
                FileGroup::new_with_source(
                    id,
                    inputs
                        .iter()
                        .map(|i| i.to_workspace_relative(project_source)),
                )?,
            );
        }

        Ok(file_groups)
    }

    async fn build_tasks(&mut self) -> miette::Result<BTreeMap<Id, Task>> {
        trace!(id = self.id.as_str(), "Building tasks");

        let mut tasks_builder = TasksBuilder::new(
            self.id.as_ref(),
            self.source.as_str(),
            &self.platform,
            TasksBuilderContext {
                detect_platform: self.context.detect_platform,
                legacy_task_inheritance: self.context.legacy_task_inheritance,
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
