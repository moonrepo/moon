use crate::codegen_error::CodegenError;
use crate::template::Template;
use miette::IntoDiagnostic;
use moon_common::consts::CONFIG_TEMPLATE_FILENAME;
use moon_common::path::RelativePathBuf;
use moon_common::Id;
use moon_config::{load_template_config_template, GeneratorConfig, TemplateLocator};
use moon_env::MoonEnvironment;
use moon_process::Command;
use moon_time::now_millis;
use starbase_archive::Archiver;
use starbase_utils::{fs, net};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::spawn;
use tracing::debug;

pub struct CodeGenerator<'app> {
    config: &'app GeneratorConfig,
    moon_env: Arc<MoonEnvironment>,
    template_locations: Vec<PathBuf>,
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
            template_locations: vec![],
            workspace_root,
        }
    }

    pub async fn resolve_template_locations(&mut self) -> miette::Result<()> {
        let mut locations = vec![];
        let mut futures = vec![];

        debug!("Resolve template locations to absolute file paths");

        for locator in &self.config.templates {
            match locator {
                TemplateLocator::File { path } => {
                    locations.push(
                        RelativePathBuf::from(path)
                            .normalize()
                            .to_logical_path(self.workspace_root),
                    );
                }
                TemplateLocator::Git {
                    remote_url,
                    revision,
                } => {
                    let base_url = remote_url.trim_start_matches("/").trim_end_matches(".git");
                    let url = format!("https://{base_url}.git");
                    let template_location = self.moon_env.templates_dir.join(&base_url);

                    futures.push(spawn(clone_and_checkout_git_repository(
                        url,
                        revision.to_owned(),
                        template_location.clone(),
                    )));

                    locations.push(template_location);
                }
                TemplateLocator::Npm { package, version } => {
                    let package_slug = package.replace('@', "").replace('/', "_");
                    let version_string = version.to_string();
                    let template_location = self
                        .moon_env
                        .templates_dir
                        .join("npm")
                        .join(&package_slug)
                        .join(&version_string);
                    let temp_file = self
                        .moon_env
                        .temp_dir
                        .join(format!("{package_slug}_{version_string}.tgz",));

                    futures.push(spawn(download_and_unpack_npm_archive(
                        package.to_owned(),
                        version_string,
                        template_location.clone(),
                        temp_file,
                    )));

                    locations.push(template_location);
                }
            }
        }

        for future in futures {
            future.await.into_diagnostic()??;
        }

        self.template_locations = locations;

        Ok(())
    }

    /// Create a new template with a schema, using the first configured template path.
    /// Will error if a template of the same name already exists.
    pub fn create_template(&self, id: &str) -> miette::Result<Template> {
        let id = Id::clean(id)?;

        let Some(TemplateLocator::File { path }) = self
            .config
            .templates
            .iter()
            .find(|locator| matches!(locator, TemplateLocator::File { .. }))
        else {
            return Err(CodegenError::CreateFileSystemOnly.into());
        };

        let template_root = RelativePathBuf::from(path)
            .to_logical_path(self.workspace_root)
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
    /// the list of template locations. Will error if no match is found.
    pub fn load_template(&self, id: &str) -> miette::Result<Template> {
        let id = Id::clean(id)?;

        debug!(
            template = id.as_str(),
            locations = ?self.template_locations,
            "Attempting to find template from resolved locations",
        );

        for template_location in &self.template_locations {
            let root = template_location.join(id.as_str());

            if !root.exists() {
                continue;
            }

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
                    extends.push(self.load_template(extend_id)?);
                }

                for extend in extends {
                    template.extend_template(extend);
                }
            }

            return Ok(template);
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
}

async fn clone_and_checkout_git_repository(
    url: String,
    revision: String,
    template_location: PathBuf,
) -> miette::Result<()> {
    debug!(
        url,
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

    Ok(())
}

async fn download_and_unpack_npm_archive(
    package: String,
    version: String,
    template_location: PathBuf,
    temp_file: PathBuf,
) -> miette::Result<()> {
    debug!(
        package,
        version, "Resolving template location for npm package"
    );

    if template_location.exists() {
        debug!(location = ?template_location, "Template location already exists locally");

        return Ok(());
    }

    let tarball_url = if let Some(index) = package.find('/') {
        // With scope: https://registry.npmjs.org/@moonrepo/cli/-/cli-1.22.7.tgz
        format!(
            "https://registry.npmjs.org/{package}/-/{}-{version}.tgz",
            &package[index + 1..]
        )
    } else {
        // Without scope: https://registry.npmjs.org/npm/-/npm-10.5.0.tgz
        format!("https://registry.npmjs.org/{package}/-/{package}-{version}.tgz")
    };

    // Download tarball
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

    Ok(())
}
