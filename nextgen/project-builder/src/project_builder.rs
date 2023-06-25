use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{color, consts, Id};
use moon_config::{
    DependencyConfig, InheritedTasksManager, InheritedTasksResult, LanguageType, PlatformType,
    ProjectConfig, ProjectDependsOn, TaskConfig,
};
use moon_file_group::FileGroup;
use moon_project::{Project, ProjectError};
use moon_task::Task;
use moon_task_builder::TasksBuilder;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::debug;

pub type LanguageDetector = dyn Fn(&Path) -> LanguageType;

pub struct ProjectBuilder<'app> {
    id: &'app str,
    source: &'app str,
    project_root: PathBuf,
    workspace_root: &'app Path,

    // Configs to derive information from
    global_config: Option<InheritedTasksResult>,
    local_config: Option<ProjectConfig>,

    // Values to be continually built
    pub language: LanguageType,
    language_detector: Option<Box<LanguageDetector>>,

    pub platform: PlatformType,
}

impl<'app> ProjectBuilder<'app> {
    pub fn new(
        id: &'app str,
        source: &'app str,
        workspace_root: &'app Path,
    ) -> Result<Self, ProjectError> {
        debug!(id, source, "Building project {} from source", color::id(id),);

        let project_root = workspace_root.join(source);

        if !project_root.exists() {
            return Err(ProjectError::MissingProjectAtSource(source.to_owned()));
        }

        Ok(ProjectBuilder {
            id,
            project_root,
            source,
            workspace_root,
            global_config: None,
            local_config: None,
            language: LanguageType::Unknown,
            language_detector: None,
            platform: PlatformType::Unknown,
        })
    }

    /// Register a function to detect a project's language when unknown.
    pub fn detect_language<F>(&mut self, detector: F) -> &mut Self
    where
        F: Fn(&Path) -> LanguageType + 'static,
    {
        self.language_detector = Some(Box::new(detector));
        self
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

        debug!(
            id = self.id,
            lookup = ?global_config.order,
            "Inheriting global file groups and tasks",
        );

        self.global_config = Some(global_config);

        Ok(self)
    }

    /// Load a `moon.yml` config file from the root of the project (derived from source).
    /// Once loaded, detect applicable language and platform fields.
    pub fn load_local_config(&mut self) -> miette::Result<&mut Self> {
        let config_name = PathBuf::from(self.source).join(consts::CONFIG_PROJECT_FILENAME);
        let config_path = self.workspace_root.join(&config_name);

        debug!(
            id = self.id,
            file = ?config_path,
            "Attempting to load {} (optional)",
            color::file(config_name.to_str().unwrap_or_default())
        );

        let config = ProjectConfig::load(self.workspace_root, config_path)?;

        // Use configured language or detect from environment
        self.language = if config.language == LanguageType::Unknown {
            if let Some(detector) = &self.language_detector {
                let language = detector(&self.project_root);

                debug!(
                    id = self.id,
                    language = ?language,
                    "Unknown project language, detecting from environment",
                );

                language
            } else {
                config.language.clone()
            }
        } else {
            config.language.clone()
        };

        // Use configured platform or infer from language
        self.platform = config.platform.unwrap_or_else(|| {
            let platform: PlatformType = self.language.clone().into();

            debug!(
                id = self.id,
                language = ?self.language,
                platform = ?self.platform,
                "Unknown tasks platform, inferring from language",
            );

            platform
        });

        self.local_config = Some(config);

        Ok(self)
    }

    /// Extend the builder with a project dependency implicitly derived from the project graph.
    /// Implicit dependencies *must not* override explicitly configured dependencies.
    pub fn extend_with_dependency(&mut self, config: DependencyConfig) -> &mut Self {
        if let Some(local_config) = &mut self.local_config {
            let has_dep = local_config.depends_on.iter().any(|d| match d {
                ProjectDependsOn::String(id) => id == &config.id,
                ProjectDependsOn::Object(cfg) => cfg.id == config.id,
            });

            if !has_dep {
                local_config
                    .depends_on
                    .push(ProjectDependsOn::Object(config));
            }
        }

        self
    }

    /// Extend the builder with a platform specific task implicitly derived from the project graph.
    /// Implicit tasks *must not* override explicitly configured tasks.
    pub fn extend_with_task(&mut self, id: Id, config: TaskConfig) -> &mut Self {
        if let Some(local_config) = &mut self.local_config {
            local_config.tasks.entry(id).or_insert(config);
        }

        self
    }

    #[tracing::instrument(name = "project", skip_all)]
    pub fn build(mut self) -> miette::Result<Project> {
        let mut project = Project {
            dependencies: self.build_dependencies()?,
            file_groups: self.build_file_groups()?,
            tasks: self.build_tasks()?,
            id: Id::raw(self.id),
            language: self.language,
            platform: self.platform,
            root: self.project_root,
            source: WorkspaceRelativePathBuf::from(self.source),
            ..Project::default()
        };

        project.inherited = self.global_config.take();

        let config = self.local_config.take().unwrap_or_default();

        project.type_of = config.type_of;
        project.config = config;

        Ok(project)
    }

    fn build_dependencies(&self) -> miette::Result<FxHashMap<Id, DependencyConfig>> {
        let mut deps = FxHashMap::default();

        debug!(id = self.id, "Building project dependencies");

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

            debug!(
                id = self.id,
                deps = ?deps.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                "Depends on {} projects",
                deps.len(),
            );
        }

        Ok(deps)
    }

    fn build_file_groups(&self) -> miette::Result<FxHashMap<Id, FileGroup>> {
        let mut file_inputs = FxHashMap::default();
        let project_source = &self.source;

        debug!(id = self.id, "Building file groups");

        // Inherit global first
        if let Some(global) = &self.global_config {
            debug!(
                id = self.id,
                groups = ?global.config.file_groups.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
                "Inheriting global file groups",
            );

            for (id, inputs) in &global.config.file_groups {
                file_inputs.insert(id, inputs);
            }
        }

        // Override with local second
        if let Some(local) = &self.local_config {
            debug!(
                id = self.id,
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

    fn build_tasks(&self) -> miette::Result<BTreeMap<Id, Task>> {
        debug!(id = self.id, "Building tasks");

        let mut tasks_builder =
            TasksBuilder::new(self.id, self.source, &self.platform, self.workspace_root);

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

        tasks_builder.build()
    }
}
