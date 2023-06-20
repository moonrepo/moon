use moon_common::{color, consts};
use moon_config::VcsConfig;
use moon_vcs::BoxedVcs;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use tracing::debug;

pub enum ShellType {
    Bash,
    Pwsh,
    PowerShell,
}

impl ShellType {
    // Determine whether we should use Bash or PowerShell as the hook file format.
    // On Unix machines, always use Bash. On Windows, scan PATH for applicable PowerShell.
    pub fn detect() -> Self {
        if cfg!(unix) {
            return ShellType::Bash;
        }

        if let Some(path_list) = env::var_os("PATH") {
            for path_dir in env::split_paths(&path_list) {
                if path_dir.join("pwsh").exists() || path_dir.join("pwsh.exe").exists() {
                    return ShellType::Pwsh;
                }
            }
        }

        ShellType::PowerShell
    }
}

pub struct HooksGenerator<'app> {
    config: &'app VcsConfig,
    output_dir: PathBuf,
    shell: ShellType,
    vcs: &'app BoxedVcs,
}

impl<'app> HooksGenerator<'app> {
    pub fn new(workspace_root: &Path, vcs: &'app BoxedVcs, config: &'app VcsConfig) -> Self {
        Self {
            config,
            output_dir: workspace_root.join(consts::CONFIG_DIRNAME).join("hooks"),
            shell: ShellType::detect(),
            vcs,
        }
    }

    pub async fn generate(&self) -> miette::Result<()> {
        debug!("Generating {} hooks", self.config.manager);

        self.sync_to_vcs(self.create_hooks()?).await?;

        Ok(())
    }

    fn create_hooks(&self) -> miette::Result<FxHashMap<&'app String, PathBuf>> {
        let mut hooks = FxHashMap::default();

        for (hook_name, commands) in &self.config.hooks {
            if commands.is_empty() {
                continue;
            }

            let hook_path = self
                .output_dir
                .join(if matches!(self.shell, ShellType::Bash) {
                    format!("{}.sh", hook_name)
                } else {
                    format!("{}.ps1", hook_name)
                });

            debug!(file = ?hook_path, "Creating {} hook", color::file(hook_name));

            self.create_hook_file(&hook_path, commands, true)?;

            hooks.insert(hook_name, hook_path);
        }

        Ok(hooks)
    }

    async fn sync_to_vcs(&self, hooks: FxHashMap<&'app String, PathBuf>) -> miette::Result<()> {
        let hooks_dir = self.vcs.get_hooks_dir().await?;
        let repo_root = self.vcs.get_repository_root().await?;

        for (hook_name, internal_path) in hooks {
            let external_path = hooks_dir.join(hook_name);

            let external_command = PathBuf::from(".")
                .join(internal_path.strip_prefix(&repo_root).unwrap())
                .display()
                .to_string();

            debug!(
                external_file = ?external_path,
                internal_file = ?internal_path,
                "Syncing local {} hook to {}",
                color::file(hook_name),
                self.config.manager,
            );

            // On Windows, the hook file itself is extensionless, which means we can't use PowerShell.
            // Instead we will execute our .ps1 script through PowerShell.
            // https://stackoverflow.com/questions/5629261/running-powershell-scripts-as-git-hooks
            #[cfg(windows)]
            {
                let powershell_exe = if matches!(self.shell, ShellType::Pwsh) {
                    "pwsh.exe"
                } else {
                    "powershell.exe"
                };

                // pre-commit
                self.create_file(
                    &external_path,
                    format!(
                        "#!/bin/sh\n{} -NoProfile -ExecutionPolicy Bypass -File '{}'",
                        powershell_exe, external_command
                    ),
                )?;
            }

            // On Unix, we can use the hook file itself and run Bash commands within it.
            #[cfg(not(windows))]
            {
                // pre-commit
                self.create_hook_file(&external_path, &[external_command], false)?;
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

        if matches!(self.shell, ShellType::Bash) {
            contents.extend(["#!/usr/bin/env bash", "set -eo pipefail", ""]);
        } else {
            contents.extend([
                if matches!(self.shell, ShellType::Pwsh) {
                    "#!/usr/bin/env pwsh"
                } else {
                    "#!/usr/bin/env powershell"
                },
                "$ErrorActionPreference = 'Stop'",
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
        }

        self.create_file(file_path, contents.join("\n"))?;

        Ok(())
    }
}
