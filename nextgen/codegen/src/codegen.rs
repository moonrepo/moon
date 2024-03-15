use crate::codegen_error::CodegenError;
use crate::template::Template;
use async_recursion::async_recursion;
use moon_common::consts::CONFIG_TEMPLATE_FILENAME;
use moon_common::path::RelativePathBuf;
use moon_common::Id;
use moon_config::{load_template_config_template, GeneratorConfig, TemplateLocator, Version};
use moon_env::MoonEnvironment;
use moon_process::Command;
use moon_time::now_millis;
use starbase_archive::Archiver;
use starbase_utils::{fs, net};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

pub struct CodeGenerator<'app> {
    config: &'app GeneratorConfig,
    moon_env: Arc<MoonEnvironment>,
    workspace_root: &'app Path,
}

impl<'app> CodeGenerator<'app> {
    pub fn new(
        workspace_root: &'app Path,
        config: &'app GeneratorConfig,
        moon_env: Arc<MoonEnvironment>,
    ) -> CodeGenerator<'app> {
        debug!(
            locations = ?config.templates.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
            "Creating code generator with template locations",
        );

        CodeGenerator {
            config,
            moon_env,
            workspace_root,
        }
    }

    /// Create a new template with a schema, using the first configured template path.
    /// Will error if a template of the same name already exists.
    pub async fn create_template(&self, id: &str) -> miette::Result<Template> {
        let id = Id::clean(id)?;

        let Some(file_locator) = self
            .config
            .templates
            .iter()
            .find(|locator| matches!(locator, TemplateLocator::File { .. }))
        else {
            return Err(CodegenError::CreateFileSystemOnly.into());
        };

        let template_root = self
            .resolve_templates_location(&file_locator)
            .await?
            .join(id.as_str());

        if template_root.exists() {
            return Err(CodegenError::ExistingTemplate(id, template_root).into());
        }

        debug!(
            template = id.as_str(),
            to = ?template_root,
            "Creating new template",
        );

        fs::write_file(
            template_root.join(CONFIG_TEMPLATE_FILENAME),
            load_template_config_template(),
        )?;

        Template::new(id, template_root)
    }

    /// Load the template with the provided name, using the first match amongst
    /// the list of template paths. Will error if no match is found.
    #[async_recursion]
    pub async fn load_template(&self, id: &str) -> miette::Result<Template> {
        let id = Id::clean(id)?;

        debug!(
            template = id.as_str(),
            "Attempting to find template from configured locations",
        );

        for template_path in &self.config.templates {
            let root = self
                .resolve_templates_location(template_path)
                .await?
                .join(id.as_str());

            if root.exists() {
                debug!(
                    template = id.as_str(),
                    root = ?root,
                    "Found template"
                );

                let mut template = Template::new(id, root)?;

                // Inherit other templates
                if !template.config.extends.is_empty() {
                    debug!(
                        template = template.id.as_str(),
                        extends = ?template.config.extends,
                        "Extending from other templates",
                    );

                    let mut extends = vec![];

                    for extend_id in &template.config.extends {
                        extends.push(self.load_template(extend_id).await?);
                    }

                    for extend in extends {
                        template.extend_template(extend);
                    }
                }

                return Ok(template);
            }
        }

        Err(CodegenError::MissingTemplate(id).into())
    }

    pub fn generate(&self, template: &Template) -> miette::Result<()> {
        debug!(template = template.id.as_str(), "Generating template files");

        for file in template.files.values() {
            if file.should_write() {
                template.write_file(file)?;
            }
        }

        for asset in template.assets.values() {
            template.copy_asset(asset)?;
        }

        debug!(template = template.id.as_str(), "Code generation complete!");

        Ok(())
    }

    async fn resolve_templates_location(
        &self,
        locator: &TemplateLocator,
    ) -> miette::Result<PathBuf> {
        match locator {
            TemplateLocator::File { path } => Ok(RelativePathBuf::from(path)
                .normalize()
                .to_logical_path(self.workspace_root)),
            TemplateLocator::Git {
                remote_url,
                revision,
            } => {
                self.clone_and_checkout_git_repository(remote_url, revision)
                    .await
            }
            TemplateLocator::Npm { package, version } => {
                self.download_and_unpack_npm_archive(package, version).await
            }
        }
    }

    async fn clone_and_checkout_git_repository(
        &self,
        remote_url: &str,
        revision: &str,
    ) -> miette::Result<PathBuf> {
        let base_url = remote_url.trim_start_matches("/").trim_end_matches(".git");
        let url = format!("https://{base_url}.git");
        let template_location = self.moon_env.templates_dir.join(&base_url);

        debug!(
            url = &url,
            revision, "Resolving template location for Git repository",
        );

        async fn run_git(args: &[&str], cwd: &Path) -> miette::Result<()> {
            Command::new("git")
                .args(args)
                .cwd(cwd)
                .without_shell()
                .create_async()
                .exec_capture_output()
                .await?;

            Ok(())
        }

        // Clone or fetch the repository
        if template_location.exists() {
            debug!(
                location = ?template_location,
                "Repository already exists, fetching latest",
            );

            run_git(
                &["fetch", "--prune", "--no-recurse-submodules"],
                &template_location,
            )
            .await?;
        } else {
            debug!(
                location = ?template_location,
                "Cloning repository into template location",
            );

            fs::create_dir_all(&template_location)?;
            run_git(&["clone", &url, "."], &template_location).await?;
        }

        // Checkout the revision
        debug!(revision, "Checking out the configured revision");

        run_git(
            &["checkout", "-B", &revision, "--track"],
            &template_location,
        )
        .await?;

        // Checkout the revision
        debug!("Pulling latest changes");

        run_git(&["pull", "--rebase", "--prune"], &template_location).await?;

        fs::write_file(
            template_location.parent().unwrap().join(".installed-at"),
            now_millis().to_string(),
        )?;

        Ok(template_location)
    }

    async fn download_and_unpack_npm_archive(
        &self,
        package: &str,
        version: &Version,
    ) -> miette::Result<PathBuf> {
        let version_string = version.to_string();
        let package_slug = package.replace('@', "").replace('/', "_");
        let template_location = self
            .moon_env
            .templates_dir
            .join("npm")
            .join(&package_slug)
            .join(&version_string);

        debug!(
            package,
            version = &version_string,
            "Resolving template location for npm package"
        );

        if template_location.exists() {
            debug!(location = ?template_location, "Template location already exists locally");

            return Ok(template_location);
        }

        let tarball_url = if let Some(index) = package.find('/') {
            // With scope: https://registry.npmjs.org/@moonrepo/cli/-/cli-1.22.7.tgz
            format!(
                "https://registry.npmjs.org/{package}/-/{}-{version_string}.tgz",
                &package[index + 1..]
            )
        } else {
            // Without scope: https://registry.npmjs.org/npm/-/npm-10.5.0.tgz
            format!("https://registry.npmjs.org/{package}/-/{package}-{version_string}.tgz")
        };

        // Download tarball
        let temp_file = self
            .moon_env
            .temp_dir
            .join(format!("{package_slug}_{version_string}.tgz",));

        debug!(tarball_url = &tarball_url, temp_file = ?temp_file, "Downloading npm tarball");

        net::download_from_url(&tarball_url, &temp_file).await?;

        // Unpack tarball
        debug!(
            temp_file = ?temp_file,
            location = ?template_location,
            "Unpacking npm tarball into template location",
        );

        Archiver::new(&template_location, &temp_file)
            .set_prefix("package")
            .unpack_from_ext()?;

        fs::remove_file(temp_file)?;

        fs::write_file(
            template_location.join(".installed-at"),
            now_millis().to_string(),
        )?;

        Ok(template_location)
    }
}
