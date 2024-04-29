use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_common::consts::PROTO_CLI_VERSION;
use moon_config::TaskOptionAffectedFiles;
use moon_platform::PlatformManager;
use moon_process::{Command, Shell};
use moon_project::Project;
use moon_task::Task;
use moon_workspace::Workspace;
use std::path::Path;
use tracing::{debug, trace};

pub struct TaskCommand<'task> {
    project: &'task Project,
    task: &'task Task,
    working_dir: &'task Path,
    workspace: &'task Workspace,

    // To be built
    command: Command,
}

impl<'task> TaskCommand<'task> {
    pub fn new(workspace: &'task Workspace, project: &'task Project, task: &'task Task) -> Self {
        let working_dir = if task.options.run_from_workspace_root {
            &workspace.root
        } else {
            &project.root
        };

        Self {
            project,
            task,
            working_dir,
            workspace,
            command: Command::new("noop"),
        }
    }

    pub async fn build(
        mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<Command> {
        self.command = PlatformManager::read()
            .get(self.task.platform)?
            .create_run_target_command(
                context,
                self.project,
                self.task,
                node.get_runtime(),
                self.working_dir,
            )
            .await?;

        debug!(
            target = self.task.target.as_str(),
            command = self.command.bin.to_str(),
            working_dir = ?self.working_dir,
            "Creating task command to execute",
        );

        // We need to handle non-zero exit code's manually
        self.command
            .cwd(self.working_dir)
            .set_error_on_nonzero(false);

        // Order is important!
        self.inject_args(context, node);
        self.inject_env(node);
        self.inject_shell();
        self.inherit_affected(context)?;
        self.inherit_config();

        Ok(self.command)
    }

    fn inject_args(&mut self, context: &ActionContext, node: &ActionNode) {
        // Must be first!
        if let ActionNode::RunTask(inner) = node {
            trace!(
                target = self.task.target.as_str(),
                args = ?inner.args,
                "Inheriting args from dependent task"
            );

            self.command.args(&inner.args);
        }

        if context.should_inherit_args(&self.task.target) {
            trace!(
                target = self.task.target.as_str(),
                args = ?context.passthrough_args,
                "Inheriting args passed through the command line"
            );

            self.command.args(&context.passthrough_args);
        }
    }

    fn inject_env(&mut self, node: &ActionNode) {
        // Must be first!
        if let ActionNode::RunTask(inner) = node {
            trace!(
                target = self.task.target.as_str(),
                env = ?inner.env,
                "Inheriting env from dependent task"
            );

            self.command.envs(&inner.env);
        }

        self.command.env("PWD", self.working_dir);

        // moon
        self.command
            .env("MOON_CACHE_DIR", &self.workspace.cache_engine.cache_dir);
        self.command
            .env("MOON_PROJECT_ID", self.project.id.as_str());
        self.command.env("MOON_PROJECT_ROOT", &self.project.root);
        self.command
            .env("MOON_PROJECT_SOURCE", self.project.source.as_str());
        self.command.env("MOON_TARGET", &self.task.target.id);
        self.command
            .env("MOON_WORKSPACE_ROOT", &self.workspace.root);
        self.command
            .env("MOON_WORKING_DIR", &self.workspace.working_dir);
        self.command.env(
            "MOON_PROJECT_SNAPSHOT",
            self.workspace
                .cache_engine
                .state
                .get_project_snapshot_path(&self.project.id),
        );

        // proto
        self.command.env("PROTO_IGNORE_MIGRATE_WARNING", "true");
        self.command.env("PROTO_NO_PROGRESS", "true");
        self.command.env("PROTO_VERSION", PROTO_CLI_VERSION);
        self.command
            .envs(self.workspace.toolchain_config.get_version_env_vars());
    }

    fn inject_shell(&mut self) {
        if self.task.options.shell == Some(true) {
            #[cfg(unix)]
            if let Some(shell) = &self.task.options.unix_shell {
                use moon_config::TaskUnixShell;

                self.command.with_shell(match shell {
                    TaskUnixShell::Bash => Shell::new("bash"),
                    TaskUnixShell::Elvish => Shell::new("elvish"),
                    TaskUnixShell::Fish => Shell::new("fish"),
                    TaskUnixShell::Zsh => Shell::new("zsh"),
                });
            }

            #[cfg(windows)]
            if let Some(shell) = &self.task.options.windows_shell {
                use moon_config::TaskWindowsShell;

                self.command.with_shell(match shell {
                    TaskWindowsShell::Bash => Shell::new("bash"),
                    TaskWindowsShell::Pwsh => Shell::new("pwsh"),
                });
            }
        } else {
            self.command.without_shell();
        }
    }

    fn inherit_affected(&mut self, context: &ActionContext) -> miette::Result<()> {
        let Some(check_affected) = &self.task.options.affected_files else {
            return Ok(());
        };

        // Only get files when `--affected` is passed
        let mut files = if context.affected_only {
            self.task
                .get_affected_files(&context.touched_files, &self.project.source)?
        } else {
            Vec::with_capacity(0)
        };

        // If we have no files, use the task's inputs instead
        if files.is_empty() && self.task.options.affected_pass_inputs {
            files = self
                .task
                .get_input_files(&self.workspace.root)?
                .into_iter()
                .filter_map(|file| {
                    file.strip_prefix(&self.project.source)
                        .ok()
                        .map(|file| file.to_owned())
                })
                .collect();
        }

        files.sort();

        // Set an environment variable
        if matches!(
            check_affected,
            TaskOptionAffectedFiles::Env | TaskOptionAffectedFiles::Enabled(true)
        ) {
            self.command.env(
                "MOON_AFFECTED_FILES",
                if files.is_empty() {
                    ".".into()
                } else {
                    files
                        .iter()
                        .map(|file| file.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                },
            );
        }

        // Pass an argument
        if matches!(
            check_affected,
            TaskOptionAffectedFiles::Args | TaskOptionAffectedFiles::Enabled(true)
        ) {
            if files.is_empty() {
                self.command.arg_if_missing(".");
            } else {
                // Mimic relative from ("./")
                self.command
                    .args(files.iter().map(|file| format!("./{file}")));
            }
        }

        Ok(())
    }

    fn inherit_config(&mut self) {
        // Terminal colors
        if self.workspace.config.runner.inherit_colors_for_piped_tasks {
            self.command.inherit_colors();
        }
    }
}
