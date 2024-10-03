use crate::config_cache::ConfigCache;
use crate::config_finder::ConfigFinder;
use crate::project_config::{PartialProjectConfig, ProjectConfig};
use crate::template_config::TemplateConfig;
use crate::toolchain_config::ToolchainConfig;
use crate::validate::check_yml_extension;
use crate::workspace_config::WorkspaceConfig;
use moon_common::color;
use schematic::{Config, ConfigLoader as Loader};
use std::ops::Deref;
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub struct ConfigLoader {
    pub finder: ConfigFinder,
}

impl ConfigLoader {
    pub fn with_pkl() -> Self {
        Self {
            finder: ConfigFinder::with_pkl(),
        }
    }

    pub fn create_project_loader<P: AsRef<Path>>(
        &self,
        project_root: P,
    ) -> miette::Result<Loader<ProjectConfig>> {
        let project_root = project_root.as_ref();
        let mut loader = Loader::<ProjectConfig>::new();

        loader.set_help(color::muted_light(
            "https://moonrepo.dev/docs/config/project",
        ));

        self.prepare_loader(&mut loader, self.finder.get_project_files(project_root))?;

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

        self.prepare_loader(&mut loader, self.finder.get_template_files(template_root))?;

        Ok(loader)
    }

    pub fn create_toolchain_loader<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<Loader<ToolchainConfig>> {
        let workspace_root = workspace_root.as_ref();
        let mut loader = Loader::<ToolchainConfig>::new();

        loader
            .set_cacher(ConfigCache::new(workspace_root))
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/toolchain",
            ))
            .set_root(workspace_root);

        self.prepare_loader(&mut loader, self.finder.get_toolchain_files(workspace_root))?;

        Ok(loader)
    }

    pub fn create_workspace_loader<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<Loader<WorkspaceConfig>> {
        let workspace_root = workspace_root.as_ref();
        let mut loader = Loader::<WorkspaceConfig>::new();

        loader
            .set_cacher(ConfigCache::new(workspace_root))
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/workspace",
            ))
            .set_root(workspace_root);

        self.prepare_loader(&mut loader, self.finder.get_workspace_files(workspace_root))?;

        Ok(loader)
    }

    pub fn load_project_config<P: AsRef<Path>>(
        &self,
        project_root: P,
    ) -> miette::Result<ProjectConfig> {
        let result = self.create_project_loader(project_root)?.load()?;

        Ok(result.config)
    }

    pub fn load_partial_project_config<P: AsRef<Path>>(
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

    pub fn load_template_config<P: AsRef<Path>>(
        &self,
        template_root: P,
    ) -> miette::Result<TemplateConfig> {
        let result = self.create_template_loader(template_root)?.load()?;

        Ok(result.config)
    }

    pub fn load_toolchain_config<P: AsRef<Path>>(
        &self,
        workspace_root: P,
        proto_config: &proto_core::ProtoConfig,
    ) -> miette::Result<ToolchainConfig> {
        let mut result = self.create_toolchain_loader(workspace_root)?.load()?;
        result.config.inherit_proto(proto_config)?;

        Ok(result.config)
    }

    pub fn load_workspace_config<P: AsRef<Path>>(
        &self,
        workspace_root: P,
    ) -> miette::Result<WorkspaceConfig> {
        let mut result = self.create_workspace_loader(workspace_root)?.load()?;
        result.config.inherit_default_plugins();

        Ok(result.config)
    }

    pub fn prepare_loader<T: Config>(
        &self,
        loader: &mut Loader<T>,
        files: Vec<PathBuf>,
    ) -> miette::Result<()> {
        for file in files {
            if file
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
            {
                loader.file_optional(check_yml_extension(&file))?;
            } else {
                loader.file_optional(file)?;
            }
        }

        Ok(())
    }
}

impl Deref for ConfigLoader {
    type Target = ConfigFinder;

    fn deref(&self) -> &Self::Target {
        &self.finder
    }
}
