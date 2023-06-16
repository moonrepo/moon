use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{color, consts, Id};
use moon_config::{
    DependencyConfig, InheritedTasksManager, InheritedTasksResult, LanguageType, PlatformType,
    ProjectConfig, ProjectDependsOn,
};
use moon_file_group::FileGroup;
use moon_project2::Project;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct ProjectBuilder<'app> {
    id: Id,
    source: WorkspaceRelativePathBuf,
    project_root: PathBuf,
    workspace_root: &'app Path,

    // Configs to derive information from
    global_config: Option<InheritedTasksResult>,
    local_config: Option<ProjectConfig>,

    // Values to be continually built
    language: LanguageType,
    platform: PlatformType,
}

impl<'app> ProjectBuilder<'app> {
    pub fn new(id: Id, source: WorkspaceRelativePathBuf, workspace_root: &'app Path) -> Self {
        debug!(
            id = ?id,
            source = ?source,
            "Building project {} from source",
            color::id(&id),
        );

        // TODO check that project root exists

        ProjectBuilder {
            id,
            project_root: source.to_logical_path(workspace_root),
            source,
            workspace_root,
            global_config: None,
            local_config: None,
            language: LanguageType::Unknown,
            platform: PlatformType::Unknown,
        }
    }

    pub fn inherit_global_config(
        &mut self,
        tasks_manager: &InheritedTasksManager,
    ) -> miette::Result<()> {
        let config = self
            .local_config
            .as_ref()
            .expect("Local config must be loaded before global config!");

        debug!(id = ?self.id, "Inheriting global file groups and tasks");

        self.global_config = Some(tasks_manager.get_inherited_config(
            &self.platform,
            &self.language,
            &config.type_of,
            &config.tags,
        )?);

        Ok(())
    }

    pub fn load_local_config<F>(&mut self, detect_language: F) -> miette::Result<()>
    where
        F: FnOnce(&Path) -> LanguageType,
    {
        let config_name = format!("{}/{}", &self.source, consts::CONFIG_PROJECT_FILENAME);
        let config_path = self.workspace_root.join(&config_name);

        debug!(
            id = ?self.id,
            file = ?config_path.display(),
            "Attempting to load {} (optional)",
            color::file(&config_name)
        );

        let config = ProjectConfig::load(self.workspace_root, config_path)?;

        // Use configured language or detect from environment
        self.language = if config.language == LanguageType::Unknown {
            debug!(
                id = ?self.id,
                "Unknown project language, detecting from environment",
            );

            detect_language(&self.project_root)
        } else {
            config.language.clone()
        };

        // Use configured platform or infer from language
        self.platform = config.platform.unwrap_or_else(|| {
            debug!(
                id = ?self.id,
                language = ?self.language,
                "Unknown tasks platform, inferring from language",
            );

            self.language.clone().into()
        });

        self.local_config = Some(config);

        Ok(())
    }

    pub fn build(mut self) -> miette::Result<Project> {
        let mut project = Project::default();
        let config = self.local_config.take().unwrap_or_default();

        project.dependencies = self.build_dependencies()?;
        project.file_groups = self.build_file_groups()?;
        project.id = self.id;
        project.language = self.language;
        project.platform = self.platform;
        project.root = self.project_root;
        project.source = self.source;
        // project.tasks;
        project.type_of = config.type_of;
        project.config = config;

        Ok(project)
    }

    fn build_dependencies(&self) -> miette::Result<FxHashMap<Id, DependencyConfig>> {
        let mut deps = FxHashMap::default();

        debug!(id = ?self.id, "Building project dependencies");

        if let Some(local) = &self.local_config {
            for dep_on in &local.depends_on {
                match dep_on {
                    ProjectDependsOn::String(id) => {
                        deps.insert(
                            id.to_owned(),
                            DependencyConfig {
                                id: id.to_owned(),
                                ..DependencyConfig::default()
                            },
                        );
                    }
                    ProjectDependsOn::Object { id, scope } => {
                        deps.insert(
                            id.to_owned(),
                            DependencyConfig {
                                id: id.to_owned(),
                                scope: scope.to_owned(),
                                ..DependencyConfig::default()
                            },
                        );
                    }
                }
            }
        }

        Ok(deps)
    }

    fn build_file_groups(&self) -> miette::Result<FxHashMap<Id, FileGroup>> {
        let mut file_inputs = FxHashMap::default();
        let project_source = &self.source;

        debug!(id = ?self.id, "Building file groups");

        // Inherit global first
        if let Some(global) = &self.global_config {
            debug!(
                id = ?self.id,
                groups = ?global.config.file_groups.keys(),
                "Inheriting global file groups",
            );

            for (id, inputs) in &global.config.file_groups {
                file_inputs.insert(id, inputs);
            }
        }

        // Override with local second
        if let Some(local) = &self.local_config {
            debug!(
                id = ?self.id,
                groups = ?local.file_groups.keys(),
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
                    id.to_owned(),
                    inputs
                        .iter()
                        .map(|i| i.to_workspace_relative(project_source)),
                )?,
            );
        }

        Ok(file_groups)
    }
}
