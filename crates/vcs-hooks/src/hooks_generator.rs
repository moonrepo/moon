use moon_common::{color, consts, is_docker, path};
use moon_config::{VcsConfig, VcsHookFormat};
use moon_vcs::BoxedVcs;
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

pub struct HooksGenerator<'app> {
    config: &'app VcsConfig,
    output_dir: PathBuf,
    shell: ShellType,
    vcs: &'app BoxedVcs,
}

impl<'app> HooksGenerator<'app> {
    pub fn new(vcs: &'app BoxedVcs, config: &'app VcsConfig, workspace_root: &Path) -> Self {
        Self {
            config,
            output_dir: workspace_root.join(consts::CONFIG_DIRNAME).join("hooks"),
            shell: ShellType::detect(),
            vcs,
        }
    }

    #[instrument(skip_all)]
    pub async fn cleanup(self) -> miette::Result<()> {
        debug!("Cleaning up {} hooks", self.config.manager);

        let hooks_dir = self.vcs.get_hooks_dir().await?;

        for hook_name in self.config.hooks.keys() {
            let hook_path = hooks_dir.join(hook_name);

            if hook_path.exists() {
                debug!(file = ?hook_path, "Removing {} hook", color::file(hook_name));

                fs::remove_file(&hook_path)?;
            }
        }

        debug!(dir = ?self.output_dir, "Removing local hooks");

        fs::remove_dir_all(&self.output_dir)?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn generate(self) -> miette::Result<bool> {
        // Do not generate if there is no `.git` folder, otherwise this
        // will create an invalid `.git` folder, which in turn enables moon caching
        // and causes downstream issues!
        if !self.vcs.is_enabled() {
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

        debug!("Generating {} hooks", self.config.manager);

        self.sync_to_vcs(self.create_hooks()?).await?;

        Ok(true)
    }

    pub fn get_internal_hook_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![];

        for (hook_name, commands) in &self.config.hooks {
            if commands.is_empty() {
                continue;
            }

            paths.push(self.create_internal_hook_path(hook_name));
        }

        paths
    }

    fn create_internal_hook_path(&self, hook_name: &str) -> PathBuf {
        self.output_dir.join(if self.is_bash_format() {
            format!("{hook_name}.sh")
        } else {
            format!("{hook_name}.ps1")
        })
    }

    fn create_hooks(&self) -> miette::Result<FxHashMap<&'app String, PathBuf>> {
        let mut hooks = FxHashMap::default();

        for (hook_name, commands) in &self.config.hooks {
            if commands.is_empty() {
                continue;
            }

            let hook_path = self.create_internal_hook_path(hook_name);

            debug!(file = ?hook_path, "Creating {} hook", color::file(hook_name));

            self.create_hook_file(&hook_path, commands, true)?;

            hooks.insert(hook_name, hook_path);
        }

        Ok(hooks)
    }

    async fn sync_to_vcs(&self, hooks: FxHashMap<&'app String, PathBuf>) -> miette::Result<()> {
        let hooks_dir = self.vcs.get_hooks_dir().await?;
        let work_root = self.vcs.get_working_root().await?;

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
                        "{} $1 $2 $3",
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
                        "#!/bin/sh\n{} -NoLogo -NoProfile -ExecutionPolicy Bypass -File \"{}\" $1 $2 $3",
                        powershell_exe, external_command.display()
                    ),
                )?;
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
            contents.extend(["#!/usr/bin/env bash", "set -eo pipefail", ""]);
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

        self.create_file(file_path, contents.join("\n"))?;

        Ok(())
    }

    fn is_bash_format(&self) -> bool {
        self.config.hook_format == VcsHookFormat::Bash || matches!(self.shell, ShellType::Bash)
    }
}
