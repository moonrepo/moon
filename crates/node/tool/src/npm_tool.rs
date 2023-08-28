use crate::node_tool::NodeTool;
use moon_config::NpmConfig;
use moon_logger::debug;
use moon_node_lang::{npm, LockfileDependencyVersions, NPM};
use moon_process::Command;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{async_trait, get_path_env_var, load_tool_plugin, DependencyManager, Tool};
use moon_utils::is_ci;
use proto_core::{Id, PluginLoader, ProtoEnvironment, Tool as ProtoTool, VersionType};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};

pub struct NpmTool {
    pub config: NpmConfig,

    pub global: bool,

    pub tool: ProtoTool,
}

impl NpmTool {
    pub async fn new(
        proto: &ProtoEnvironment,
        config: &NpmConfig,
        plugin_loader: &PluginLoader,
    ) -> miette::Result<NpmTool> {
        Ok(NpmTool {
            global: config.version.is_none(),
            config: config.to_owned(),
            tool: load_tool_plugin(
                &Id::raw("npm"),
                proto,
                config.plugin.as_ref().unwrap(),
                plugin_loader,
            )
            .await?,
        })
    }
}

#[async_trait]
impl Tool for NpmTool {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(if self.global {
            "npm".into()
        } else {
            self.tool.get_bin_path()?.to_path_buf()
        })
    }

    fn get_shim_path(&self) -> Option<PathBuf> {
        self.tool.get_shim_path().map(|p| p.to_path_buf())
    }

    async fn setup(&mut self, last_versions: &mut FxHashMap<String, String>) -> miette::Result<u8> {
        let mut count = 0;
        let version = self.config.version.clone();

        let Some(version) = version else {
            return Ok(count);
        };

        let version_type = VersionType::parse(&version)?;

        if self.tool.is_setup(&version_type).await? {
            debug!("npm has already been setup");

            return Ok(count);
        }

        // When offline and the tool doesn't exist, fallback to the global binary
        if proto_core::is_offline() {
            debug!(
                "No internet connection and npm has not been setup, falling back to global binary in PATH"
            );

            self.global = true;

            return Ok(count);
        }

        if let Some(last) = last_versions.get("npm") {
            if last == &version && self.tool.get_tool_dir().exists() {
                return Ok(count);
            }
        }

        print_checkpoint(format!("installing npm v{version}"), Checkpoint::Setup);

        if self.tool.setup(&version_type).await? {
            last_versions.insert("npm".into(), version);
            count += 1;
        }

        Ok(count)
    }

    async fn teardown(&mut self) -> miette::Result<()> {
        self.tool.teardown().await?;

        Ok(())
    }
}

#[async_trait]
impl DependencyManager<NodeTool> for NpmTool {
    fn create_command(&self, node: &NodeTool) -> miette::Result<Command> {
        let mut cmd = if self.global {
            Command::new("npm")
        } else if let Some(shim) = self.get_shim_path() {
            Command::new(shim)
        } else {
            let mut cmd = Command::new(node.get_bin_path()?);
            cmd.arg(self.get_bin_path()?);
            cmd
        };

        if !self.global {
            cmd.env("PATH", get_path_env_var(&self.tool.get_tool_dir()));
        }

        cmd.env("PROTO_NODE_BIN", node.get_bin_path()?);

        Ok(cmd)
    }

    async fn dedupe_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        self.create_command(node)?
            .args(["dedupe"])
            .cwd(working_dir)
            .set_print_command(log)
            .create_async()
            .exec_capture_output()
            .await?;

        Ok(())
    }

    fn get_lock_filename(&self) -> String {
        String::from(NPM.lockfile)
    }

    fn get_manifest_filename(&self) -> String {
        String::from(NPM.manifest)
    }

    async fn get_resolved_dependencies(
        &self,
        project_root: &Path,
    ) -> miette::Result<LockfileDependencyVersions> {
        let Some(lockfile_path) = fs::find_upwards(NPM.lockfile, project_root) else {
            return Ok(FxHashMap::default());
        };

        Ok(npm::load_lockfile_dependencies(lockfile_path)?)
    }

    async fn install_dependencies(
        &self,
        node: &NodeTool,
        working_dir: &Path,
        log: bool,
    ) -> miette::Result<()> {
        let mut args = vec!["install"];

        if is_ci() {
            let lockfile = working_dir.join(self.get_lock_filename());

            // npm will error if using `ci` and a lockfile does not exist!
            if lockfile.exists() {
                args.clear();
                args.push("ci");
            }
        } else {
            args.push("--no-audit");
        }

        args.push("--no-fund");

        let mut cmd = self.create_command(node)?;

        cmd.args(args).cwd(working_dir).set_print_command(log);

        let mut cmd = cmd.create_async();

        if env::var("MOON_TEST_HIDE_INSTALL_OUTPUT").is_ok() {
            cmd.exec_capture_output().await?;
        } else {
            cmd.exec_stream_output().await?;
        }

        Ok(())
    }

    async fn install_focused_dependencies(
        &self,
        node: &NodeTool,
        package_names: &[String],
        production_only: bool,
    ) -> miette::Result<()> {
        let mut cmd = self.create_command(node)?;
        cmd.args(["install"]);

        if production_only {
            cmd.arg("--production");
        }

        for package_name in package_names {
            cmd.args(["--workspace", package_name]);
        }

        cmd.create_async().exec_stream_output().await?;

        Ok(())
    }
}
