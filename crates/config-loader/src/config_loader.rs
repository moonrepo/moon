use crate::config_cache::ConfigCache;
use crate::config_finder::ConfigFinder;
use crate::extensions_config_ext::ExtensionsConfigExt;
use crate::formats::hcl::HclFormat;
use crate::toolchains_config_ext::ToolchainsConfigExt;
use moon_common::color;
use moon_config::{
    ExtensionsConfig, InheritedTasksConfig, InheritedTasksManager, PartialProjectConfig,
    ProjectConfig, TemplateConfig, TemplateFrontmatterConfig, ToolchainsConfig, WorkspaceConfig,
};
use proto_core::{PluginLocator, ProtoConfig};
use schematic::{Config, ConfigLoader as Loader};
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// The project config file located within a project root.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ProjectConfigFile {
    /// Not located, so every supported file name will be probed when loading.
    #[default]
    Unresolved,

    /// The project root does not exist.
    MissingRoot,

    /// The project root exists, with the config file of the
    /// highest extension precedence that it contains, if any.
    Located(Option<PathBuf>),
}

#[derive(Clone, Debug, Default)]
pub struct ConfigLoader {
    pub dir: PathBuf, // .moon
    pub dir_prefix: String,
    finder: ConfigFinder,
}

impl ConfigLoader {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    pub fn locate_dir(&mut self, workspace_root: &Path) -> PathBuf {
        let moon_dir = workspace_root.join(".moon");
        let config_moon_dir = workspace_root.join(".config").join("moon");

        if config_moon_dir.exists() {
            self.dir = config_moon_dir;
            self.dir_prefix = ".config/moon".into();
        } else {
            self.dir = moon_dir;
            self.dir_prefix = ".moon".into();
        }

        self.dir.clone()
    }

    pub fn create_extensions_loader<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<Loader<ExtensionsConfig>> {
        let mut loader = Loader::<ExtensionsConfig>::new();

        loader
            .set_cacher(ConfigCache::new(&self.dir))
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/extensions",
            ))
            .set_root(workspace_root);

        self.prepare_loader(&mut loader, self.get_extensions_files())?;

        Ok(loader)
    }

    pub fn create_project_loader<P: AsRef<Path>>(
        &self,
        project_root: P,
    ) -> miette::Result<Loader<ProjectConfig>> {
        self.create_project_loader_with_files(self.get_project_files(project_root.as_ref()))
    }

    fn create_project_loader_with_files(
        &self,
        files: Vec<PathBuf>,
    ) -> miette::Result<Loader<ProjectConfig>> {
        let mut loader = Loader::<ProjectConfig>::new();

        loader.set_help(color::muted_light(
            "https://moonrepo.dev/docs/config/project",
        ));

        self.prepare_loader(&mut loader, files)?;

        Ok(loader)
    }

    pub fn create_tasks_loader<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<Loader<InheritedTasksConfig>> {
        let mut loader = Loader::<InheritedTasksConfig>::new();

        loader
            .set_cacher(ConfigCache::new(&self.dir))
            .set_help(color::muted_light("https://moonrepo.dev/docs/config/tasks"))
            .set_root(workspace_root);

        // Do not prepare since there are many tasks paths!

        Ok(loader)
    }

    pub fn create_template_loader<P: AsRef<Path>>(
        &self,
        template_root: P,
    ) -> miette::Result<Loader<TemplateConfig>> {
        let template_root = template_root.as_ref();
        let mut loader = Loader::<TemplateConfig>::new();

        loader.set_help(color::muted_light(
            "https://moonrepo.dev/docs/config/template",
        ));

        self.prepare_loader(&mut loader, self.get_template_files(template_root))?;

        Ok(loader)
    }

    pub fn create_toolchains_loader<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<Loader<ToolchainsConfig>> {
        let mut loader = Loader::<ToolchainsConfig>::new();

        loader
            .set_cacher(ConfigCache::new(&self.dir))
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/toolchain",
            ))
            .set_root(workspace_root);

        self.prepare_loader(&mut loader, self.get_toolchains_files())?;

        Ok(loader)
    }

    pub fn create_workspace_loader<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<Loader<WorkspaceConfig>> {
        let mut loader = Loader::<WorkspaceConfig>::new();

        loader
            .set_cacher(ConfigCache::new(&self.dir))
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/workspace",
            ))
            .set_root(workspace_root);

        self.prepare_loader(&mut loader, self.get_workspace_files())?;

        Ok(loader)
    }

    pub fn load_extensions_config<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<ExtensionsConfig> {
        let mut result = self.create_extensions_loader(workspace_root)?.load()?;

        ExtensionsConfigExt::inherit_defaults(&mut result.config)?;

        // Resolve plugin file locations
        for config in result.config.plugins.values_mut() {
            if let Some(PluginLocator::File(file)) = &mut config.plugin {
                let file_path = file.get_unresolved_path();

                file.path = Some(if file_path.is_absolute() {
                    file_path
                } else {
                    self.dir.join(file_path)
                });
            }
        }

        Ok(result.config)
    }

    pub fn load_project_config<P: AsRef<Path>>(
        &self,
        project_root: P,
    ) -> miette::Result<ProjectConfig> {
        let result = self.create_project_loader(project_root)?.load()?;

        Ok(result.config)
    }

    pub fn load_project_partial_config<P: AsRef<Path>>(
        &self,
        project_root: P,
    ) -> miette::Result<PartialProjectConfig> {
        let result = self
            .create_project_loader(project_root)?
            .load_partial(&())?;

        Ok(result)
    }

    pub fn load_project_config_from_source<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        workspace_root: P,
        project_source: S,
    ) -> miette::Result<ProjectConfig> {
        let workspace_root = workspace_root.as_ref();
        let project_root = workspace_root.join(project_source.as_ref());

        let result = self
            .create_project_loader(project_root)?
            .set_root(workspace_root)
            .load()?;

        Ok(result.config)
    }

    pub fn load_project_config_from_file<P: AsRef<Path>>(
        &self,
        workspace_root: P,
        file: Option<&Path>,
    ) -> miette::Result<ProjectConfig> {
        let result = self
            .create_project_loader_with_files(file.map(Path::to_path_buf).into_iter().collect())?
            .set_root(workspace_root)
            .load()?;

        Ok(result.config)
    }

    pub fn load_tasks_config_from_path<T: AsRef<Path>, P: AsRef<Path>>(
        &self,
        workspace_root: T,
        path: P,
    ) -> miette::Result<InheritedTasksConfig> {
        let mut loader = self.create_tasks_loader(workspace_root)?;

        self.prepare_loader(&mut loader, vec![path.as_ref().to_path_buf()])?;

        Ok(loader.load()?.config)
    }

    pub fn load_tasks_manager<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<InheritedTasksManager> {
        self.load_tasks_manager_from(workspace_root, &self.dir)
    }

    pub fn load_tasks_manager_from<P: AsRef<Path>, D: AsRef<Path>>(
        &self,
        workspace_root: P,
        config_dir: D,
    ) -> miette::Result<InheritedTasksManager> {
        let workspace_root = workspace_root.as_ref();
        let config_dir = config_dir.as_ref();
        let mut manager = InheritedTasksManager::default();

        // tasks/**/*.*
        for file in self.get_tasks_files(config_dir)? {
            if file.exists() {
                manager.add_config(
                    workspace_root,
                    &file,
                    self.load_tasks_config_from_path(workspace_root, &file)?,
                )?;
            }
        }

        Ok(manager)
    }

    pub fn load_template_config<P: AsRef<Path>>(
        &self,
        template_root: P,
    ) -> miette::Result<TemplateConfig> {
        let result = self.create_template_loader(template_root)?.load()?;

        Ok(result.config)
    }

    pub fn load_toolchains_config<P: AsRef<Path>>(
        &self,
        workspace_root: P,
        proto_config: &ProtoConfig,
    ) -> miette::Result<ToolchainsConfig> {
        let mut result = self.create_toolchains_loader(workspace_root)?.load()?;
        result.config.inherit_versions_from_env_vars()?;

        ToolchainsConfigExt::inherit_defaults(&mut result.config, proto_config)?;

        // Resolve plugin file locations
        for config in result.config.plugins.values_mut() {
            if let Some(PluginLocator::File(file)) = &mut config.plugin {
                let file_path = file.get_unresolved_path();

                file.path = Some(if file_path.is_absolute() {
                    file_path
                } else {
                    self.dir.join(file_path)
                });
            }
        }

        Ok(result.config)
    }

    pub fn load_workspace_config<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<WorkspaceConfig> {
        let result = self.create_workspace_loader(workspace_root)?.load()?;

        Ok(result.config)
    }

    pub fn parse_template_frontmatter_config<T: AsRef<str>>(
        content: T,
    ) -> miette::Result<TemplateFrontmatterConfig> {
        let mut content = content.as_ref();

        if content.is_empty() {
            content = "{}";
        }

        let result = Loader::<TemplateFrontmatterConfig>::new()
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/template",
            ))
            .code(content, "frontmatter.yml")?
            .load()?;

        Ok(result.config)
    }

    pub fn prepare_loader<T: Config>(
        &self,
        loader: &mut Loader<T>,
        files: Vec<PathBuf>,
    ) -> miette::Result<()> {
        loader.add_format(HclFormat::default());

        for file in files {
            loader.file_optional(file)?;
        }

        Ok(())
    }

    pub fn get_debug_label(&self, name: &str) -> String {
        self.finder.get_debug_label(name)
    }

    pub fn get_debug_label_root(&self, name: &str) -> String {
        self.finder.get_debug_label_root(name, &self.dir)
    }

    pub fn get_extensions_files(&self) -> Vec<PathBuf> {
        self.finder
            .get_extensions_file_names()
            .into_iter()
            .map(|name| self.dir.join(name))
            .collect()
    }

    pub fn get_project_files(&self, project_root: &Path) -> Vec<PathBuf> {
        self.finder
            .get_project_file_names()
            .into_iter()
            .map(|name| project_root.join(name))
            .collect()
    }

    /// Locate the config file within the project root, by probing each supported
    /// file name in extension precedence order, and stopping at the first that
    /// exists. When none exist, also check whether the project root exists.
    ///
    /// Probing (instead of reading the directory) is cheaper for the common
    /// case of a `moon.yml` file, and resolves file names the same way as the
    /// file system (for example, case-insensitively on macOS and Windows).
    pub fn locate_project_file(&self, project_root: &Path) -> ProjectConfigFile {
        for name in self.get_project_file_names() {
            let file = project_root.join(name);

            if file.exists() {
                return ProjectConfigFile::Located(Some(file));
            }
        }

        if project_root.exists() {
            ProjectConfigFile::Located(None)
        } else {
            ProjectConfigFile::MissingRoot
        }
    }

    pub fn get_tasks_files(&self, tasks_dir: &Path) -> miette::Result<Vec<PathBuf>> {
        self.finder.get_from_dir(tasks_dir.join("tasks"))
    }

    pub fn get_template_files(&self, template_root: &Path) -> Vec<PathBuf> {
        self.finder
            .get_template_file_names()
            .into_iter()
            .map(|name| template_root.join(name))
            .collect()
    }

    pub fn get_toolchains_files(&self) -> Vec<PathBuf> {
        self.finder
            .get_toolchains_file_names()
            .into_iter()
            .map(|name| self.dir.join(name))
            .collect()
    }

    pub fn get_workspace_files(&self) -> Vec<PathBuf> {
        self.finder
            .get_workspace_file_names()
            .into_iter()
            .map(|name| self.dir.join(name))
            .collect()
    }
}

impl Deref for ConfigLoader {
    type Target = ConfigFinder;

    fn deref(&self) -> &Self::Target {
        &self.finder
    }
}
