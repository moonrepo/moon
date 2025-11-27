use moon_app_context::AppContext;
use moon_cache_item::{CacheItem, cache_item};
use moon_common::{
    color, is_docker,
    path::{PathExt, WorkspaceRelativePathBuf},
};
use moon_config::{VcsConfig, VcsHookFormat};
use moon_vcs::VcsHookEnvironment;
use starbase_utils::fs;
use std::path::Path;
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
        {
            ShellType::Bash
        }

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
        pub relative_hooks_dir: Option<WorkspaceRelativePathBuf>,
    }
);

pub struct HooksGenerator<'app> {
    app_context: &'app AppContext,
    config: &'app VcsConfig,
    shell: ShellType,
}

impl<'app> HooksGenerator<'app> {
    pub fn new(app_context: &'app AppContext, config: &'app VcsConfig) -> Self {
        Self {
            app_context,
            config,
            shell: ShellType::detect(),
        }
    }

    #[instrument(skip_all)]
    pub async fn cleanup(self) -> miette::Result<()> {
        debug!("Cleaning up {} hooks", self.config.client);

        self.app_context.vcs.teardown_hooks().await?;

        let state = self.cleanup_previous_state()?;

        if let Some(dir) = &state.data.relative_hooks_dir {
            fs::remove_dir_all(dir.to_logical_path(&self.app_context.workspace_root))?;
        }

        Ok(())
    }

    fn cleanup_previous_state(&self) -> miette::Result<CacheItem<HooksState>> {
        let state = self
            .app_context
            .cache_engine
            .state
            .load_state::<HooksState>("vcsHooks.json")?;

        if let Some(dir) = &state.data.relative_hooks_dir {
            self.remove_hook_files(
                &dir.to_logical_path(&self.app_context.workspace_root),
                &state.data.hook_names,
            )?;
        }

        Ok(state)
    }

    #[instrument(skip_all)]
    pub async fn generate(self) -> miette::Result<bool> {
        let vcs = &self.app_context.vcs;

        // Do not generate if there is no `.git` folder, otherwise this
        // will create an invalid `.git` folder, which in turn enables moon caching
        // and causes downstream issues!
        if !vcs.is_enabled() {
            if is_docker() {
                warn!(
                    "In a Docker container/image and .git does not exist, not generating {} hooks",
                    self.config.client
                );
            } else {
                debug!(
                    "Not generating {} hooks as .git does not exist",
                    self.config.client
                );
            }

            return Ok(false);
        }

        let Some(env) = vcs.setup_hooks().await? else {
            return Ok(false);
        };

        let mut state = self.cleanup_previous_state()?;

        debug!("Generating {} hooks", self.config.client);

        self.create_hook_files(&env)?;

        state.data.hook_names = self.config.hooks.keys().cloned().collect();
        state.data.relative_hooks_dir = env
            .hooks_dir
            .relative_to(&self.app_context.workspace_root)
            .ok();

        state.save()?;

        Ok(true)
    }

    fn load_state(&self) -> miette::Result<CacheItem<HooksState>> {
        let state = self
            .app_context
            .cache_engine
            .state
            .load_state::<HooksState>("vcsHooks.json")?;

        Ok(state)
    }

    pub fn verify_hooks_exist(&self) -> miette::Result<bool> {
        let Some(dir) = self.load_state()?.data.relative_hooks_dir else {
            return Ok(false);
        };

        let hooks_dir = dir.to_logical_path(&self.app_context.workspace_root);

        for (hook_name, commands) in &self.config.hooks {
            if !commands.is_empty() && !hooks_dir.join(hook_name).exists() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn create_hook_files(&self, env: &VcsHookEnvironment) -> miette::Result<()> {
        for (hook_name, commands) in &self.config.hooks {
            if !commands.is_empty() {
                self.create_hook_file(env, hook_name, commands)?;
            }
        }

        Ok(())
    }

    fn create_hook_file(
        &self,
        env: &VcsHookEnvironment,
        hook_name: &str,
        commands: &[String],
    ) -> miette::Result<()> {
        let hook = self.format_hook_file(commands, true)?;

        // Bash only
        if self.is_bash_format() {
            let hook_path = env.hooks_dir.join(hook_name);

            debug!(file = ?hook_path, "Creating Bash {} hook", color::file(hook_name));

            self.write_file(&hook_path, hook)?;
        }
        // Bash + PowerShell
        else {
            let hook_path = env.hooks_dir.join(format!("{hook_name}.ps1"));

            debug!(file = ?hook_path, "Creating PowerShell {} hook", color::file(hook_name));

            self.write_file(&hook_path, hook)?;

            // Create a bash hook to call the PowerShell script
            let bash_hook_path = env.hooks_dir.join(hook_name);

            self.write_file(&bash_hook_path, format!(
                "#!/bin/sh\n{} -NoLogo -NoProfile -ExecutionPolicy Bypass -File \"{}\" $1 $2 $3 $4 $5",
                if matches!(self.shell, ShellType::Pwsh) {
                    "pwsh.exe"
                } else {
                    "powershell.exe"
                },
                hook_path.relative_to(&env.working_dir).unwrap(),
            ))?;
        }

        Ok(())
    }

    fn format_hook_file(&self, commands: &[String], with_header: bool) -> miette::Result<String> {
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

        Ok(content)
    }

    fn remove_hook_files(&self, hooks_dir: &Path, hook_names: &[String]) -> miette::Result<()> {
        for hook_name in hook_names {
            let hook_path = hooks_dir.join(hook_name);

            if hook_path.exists() {
                debug!(file = ?hook_path, "Removing {} hook", color::file(hook_name));

                fs::remove_file(&hook_path)?;
                fs::remove_file(hooks_dir.join(format!("{hook_name}.ps1")))?;
            }
        }

        Ok(())
    }

    fn is_bash_format(&self) -> bool {
        self.config.hook_format == VcsHookFormat::Bash || matches!(self.shell, ShellType::Bash)
    }

    fn write_file(&self, path: impl AsRef<Path>, contents: String) -> miette::Result<()> {
        let path = path.as_ref();

        fs::write_file(path, contents)?;
        fs::update_perms(path, Some(0o0775))?;

        Ok(())
    }
}
