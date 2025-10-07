use moon_app_context::AppContext;
use moon_cache_item::{CacheItem, cache_item};
use moon_common::{color, consts, is_docker, path};
use moon_config::{VcsConfig, VcsHookFormat};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, warn};

pub enum ShellType {
    Bash,
    Pwsh,
    PowerShell,
}

impl ShellType {
    // Determine whether we should use Bash or PowerShell as the hook file format.
    // On Unix machines, always use Bash. On Windows, scan PATH for applicable PowerShell.
    pub fn detect() -> Self {
        #[cfg(not(windows))]
        return ShellType::Bash;

        #[cfg(windows)]
        if system_env::is_command_on_path("pwsh") {
            ShellType::Pwsh
        } else {
            ShellType::PowerShell
        }
    }
}

cache_item!(
    pub struct HooksState {
        pub hook_names: Vec<String>,
    }
);

pub struct HooksGenerator<'app> {
    app_context: &'app AppContext,
    config: &'app VcsConfig,
    output_dir: PathBuf,
    shell: ShellType,
}

impl<'app> HooksGenerator<'app> {
    pub fn new(
        app_context: &'app AppContext,
        config: &'app VcsConfig,
        workspace_root: &Path,
    ) -> Self {
        Self {
            app_context,
            config,
            output_dir: workspace_root.join(consts::CONFIG_DIRNAME).join("hooks"),
            shell: ShellType::detect(),
        }
    }

    #[instrument(skip_all)]
    pub async fn cleanup(self) -> miette::Result<()> {
        debug!("Cleaning up {} hooks", self.config.manager);

        // Remove external files
        debug!(dir = ?self.output_dir, "Removing external hooks");

        self.cleanup_previous_state().await?;

        self.remove_from_vcs(&self.get_hook_names()).await?;

        // Remove internal files
        debug!(dir = ?self.output_dir, "Removing internal hooks");

        fs::remove_dir_all(&self.output_dir)?;

        Ok(())
    }

    pub async fn cleanup_previous_state(&self) -> miette::Result<CacheItem<HooksState>> {
        let state = self
            .app_context
            .cache_engine
            .state
            .load_state::<HooksState>("vcsHooks.json")?;

        self.remove_from_vcs(&state.data.hook_names).await?;

        Ok(state)
    }

    #[instrument(skip_all)]
    pub async fn generate(self) -> miette::Result<bool> {
        // Do not generate if there is no `.git` folder, otherwise this
        // will create an invalid `.git` folder, which in turn enables moon caching
        // and causes downstream issues!
        if !self.app_context.vcs.is_enabled() {
            if is_docker() {
                warn!(
                    "In a Docker container/image and .git does not exist, not generating {} hooks",
                    self.config.manager
                );
            } else {
                debug!(
                    "Not generating {} hooks as .git does not exist",
                    self.config.manager
                );
            }

            return Ok(false);
        }

        let mut state = self.cleanup_previous_state().await?;

        debug!("Generating {} hooks", self.config.manager);

        self.add_to_vcs(self.create_hook_files()?).await?;

        state.data.hook_names = self.get_hook_names();
        state.save()?;

        Ok(true)
    }

    pub async fn verify_hooks_exist(&self) -> miette::Result<bool> {
        let hooks_dir = self.app_context.vcs.get_hooks_dir().await?;

        for (hook_name, commands) in &self.config.hooks {
            if commands.is_empty() {
                continue;
            }

            if !self.get_internal_hook_path(hook_name).exists()
                || !hooks_dir.join(hook_name).exists()
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn get_internal_hook_path(&self, hook_name: &str) -> PathBuf {
        self.output_dir.join(if self.is_bash_format() {
            format!("{hook_name}.sh")
        } else {
            format!("{hook_name}.ps1")
        })
    }

    fn get_hook_names(&self) -> Vec<String> {
        self.config.hooks.keys().cloned().collect()
    }

    async fn add_to_vcs(&self, hooks: FxHashMap<&'app String, PathBuf>) -> miette::Result<()> {
        let hooks_dir = self.app_context.vcs.get_hooks_dir().await?;
        let work_root = self.app_context.vcs.get_working_root().await?;

        for (hook_name, internal_path) in hooks {
            let external_path = hooks_dir.join(hook_name);

            let external_command = match internal_path.strip_prefix(&work_root) {
                Ok(rel) => PathBuf::from(".").join(rel),
                _ => internal_path.clone(),
            };

            debug!(
                external_file = ?external_path,
                internal_file = ?internal_path,
                "Syncing local {} hook to {}",
                color::file(hook_name),
                self.config.manager,
            );

            // On Unix or a system that supports Bash, we can use the hook file
            // itself and run Bash commands within it.
            if self.is_bash_format() || cfg!(not(windows)) {
                // pre-commit
                self.create_hook_file(
                    &external_path,
                    &[format!(
                        "{} $1 $2 $3 $4 $5",
                        path::to_virtual_string(external_command)?
                    )],
                    false,
                )?;
            }
            // On Windows, the hook file itself is extensionless, which means we can't use PowerShell.
            // Instead we will execute our .ps1 script through PowerShell.
            // https://stackoverflow.com/questions/5629261/running-powershell-scripts-as-git-hooks
            else {
                let powershell_exe = if matches!(self.shell, ShellType::Pwsh) {
                    "pwsh.exe"
                } else {
                    "powershell.exe"
                };

                // pre-commit
                self.create_file(
                    &external_path,
                    format!(
                        "#!/bin/sh\n{} -NoLogo -NoProfile -ExecutionPolicy Bypass -File \"{}\" $1 $2 $3 $4 $5",
                        powershell_exe, external_command.display()
                    ),
                )?;
            }
        }

        Ok(())
    }

    async fn remove_from_vcs(&self, hook_names: &[String]) -> miette::Result<()> {
        let hooks_dir = self.app_context.vcs.get_hooks_dir().await?;

        for hook_name in hook_names {
            let hook_path = hooks_dir.join(hook_name);

            if hook_path.exists() {
                debug!(file = ?hook_path, "Removing {} hook", color::file(hook_name));

                fs::remove_file(&hook_path)?;
            }
        }

        Ok(())
    }

    fn create_file(&self, file_path: &Path, contents: String) -> miette::Result<()> {
        fs::write_file(file_path, contents)?;
        fs::update_perms(file_path, Some(0o0775))?;

        Ok(())
    }

    fn create_hook_file(
        &self,
        file_path: &Path,
        commands: &[String],
        with_header: bool,
    ) -> miette::Result<()> {
        let mut contents = vec![];

        if self.is_bash_format() {
            contents.extend([
                "#!/usr/bin/env bash",
                "set -eo pipefail",
                "",
                "for (( i=0; i <= $#; i++ )); do",
                "  declare -x \"ARG$i\"=\"${!i}\"",
                "done",
                "",
            ]);
        } else {
            contents.extend([
                if matches!(self.shell, ShellType::Pwsh) {
                    "#!/usr/bin/env pwsh"
                } else {
                    "#!/usr/bin/env powershell"
                },
                "$ErrorActionPreference = 'Stop'",
                // https://learn.microsoft.com/en-us/powershell/scripting/learn/experimental-features?view=powershell-7.4#psnativecommanderroractionpreference
                "$PSNativeCommandErrorActionPreference = $true",
                "",
                "Set-Item -Path \"Env:ARG0\" -Value \"$PSCommandPath\"",
                "for ($i = 0; $i -lt $args.Count; $i++) {",
                "  Set-Item -Path \"Env:ARG$($i + 1)\" -Value \"$($args[$i])\"",
                "}",
                "",
            ]);
        }

        if with_header {
            contents.extend([
                "# Automatically generated by moon. DO NOT MODIFY!",
                "# https://moonrepo.dev/docs/guides/vcs-hooks",
                "",
            ]);
        }

        for command in commands {
            contents.push(command);

            // https://github.com/moonrepo/moon/issues/1761
            if !self.is_bash_format() {
                contents.extend([
                    "",
                    "if ($LASTEXITCODE -ne 0) {",
                    "  exit $LASTEXITCODE",
                    "}",
                    "",
                ]);
            }
        }

        contents.push("\n");

        let mut content = contents.join("\n");

        if !self.is_bash_format() {
            content = content.replace("$ARG", "$env:ARG");
        }

        self.create_file(file_path, content)?;

        Ok(())
    }

    fn create_hook_files(&self) -> miette::Result<FxHashMap<&'app String, PathBuf>> {
        let mut hooks = FxHashMap::default();

        fs::remove_dir_all(&self.output_dir)?;

        for (hook_name, commands) in &self.config.hooks {
            if commands.is_empty() {
                continue;
            }

            let hook_path = self.get_internal_hook_path(hook_name);

            debug!(file = ?hook_path, "Creating {} hook", color::file(hook_name));

            self.create_hook_file(&hook_path, commands, true)?;

            hooks.insert(hook_name, hook_path);
        }

        Ok(hooks)
    }

    fn is_bash_format(&self) -> bool {
        self.config.hook_format == VcsHookFormat::Bash || matches!(self.shell, ShellType::Bash)
    }
}
