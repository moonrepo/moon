use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::path::PathExt;
use moon_config::TaskOptionAffectedFiles;
use moon_env_var::GlobalEnvBag;
use moon_process::{Command, Shell, ShellType};
use moon_process_augment::CommandAugmenter;
use moon_project::Project;
use moon_task::Task;
use std::path::Path;
use tracing::{debug, instrument, trace};

pub struct CommandBuilder<'task> {
    app: &'task AppContext,
    node: &'task ActionNode,
    project: &'task Project,
    task: &'task Task,
    working_dir: &'task Path,
    env_bag: &'task GlobalEnvBag,

    // To be built
    command: Command,
}

impl<'task> CommandBuilder<'task> {
    pub fn new(
        app: &'task AppContext,
        project: &'task Project,
        task: &'task Task,
        node: &'task ActionNode,
    ) -> Self {
        let working_dir = if task.options.run_from_workspace_root {
            &app.workspace_root
        } else {
            &project.root
        };

        Self {
            app,
            node,
            project,
            task,
            working_dir,
            env_bag: GlobalEnvBag::instance(),
            command: Command::new("noop"),
        }
    }

    pub fn set_env_bag(&mut self, bag: &'task GlobalEnvBag) {
        self.env_bag = bag;
    }

    #[instrument(name = "build_command", skip_all)]
    pub async fn build(mut self, context: &ActionContext, hash: &str) -> miette::Result<Command> {
        debug!(
            task_target = self.task.target.as_str(),
            working_dir = ?self.working_dir,
            "Creating task child process to execute",
        );

        self.command = self.build_command().await?;

        // We need to handle non-zero exit code's manually
        self.command
            .cwd(self.working_dir)
            .set_error_on_nonzero(false);

        // Order is important!
        self.inject_args(context);
        self.inject_env(hash);
        self.inject_shell();
        self.inherit_affected(context)?;
        self.inherit_config();

        // Must be last!
        self.command.inherit_path()?;

        Ok(self.command)
    }

    async fn build_command(&mut self) -> miette::Result<Command> {
        let mut augment = CommandAugmenter::from_task(self.app, self.env_bag, self.task);
        augment
            .inherit_from_plugins(Some(self.project), Some(self.task))
            .await?;

        let mut command = augment.create_command();

        // Scripts should be used as-is
        command.escape_args = self.task.script.is_none();

        Ok(command)
    }

    #[instrument(skip_all)]
    fn inject_args(&mut self, context: &ActionContext) {
        // Must be first!
        if let ActionNode::RunTask(inner) = &self.node
            && !inner.args.is_empty()
        {
            trace!(
                task_target = self.task.target.as_str(),
                args = ?inner.args,
                "Inheriting args from dependent task"
            );

            self.command.args(&inner.args);
        }

        if self.task.script.is_none()
            && context.should_inherit_args(&self.task.target)
            && !context.passthrough_args.is_empty()
        {
            trace!(
                task_target = self.task.target.as_str(),
                args = ?context.passthrough_args,
                "Inheriting args passed through the command line"
            );

            self.command.args(&context.passthrough_args);
        }
    }

    #[instrument(skip_all)]
    fn inject_env(&mut self, hash: &str) {
        // Must be first!
        if let ActionNode::RunTask(inner) = &self.node
            && !inner.env.is_empty()
        {
            trace!(
                task_target = self.task.target.as_str(),
                env = ?inner.env,
                "Inheriting env from dependent task"
            );

            self.command.envs(&inner.env);
        }

        self.command.env("PWD", self.working_dir);

        // moon
        self.command
            .env("MOON_CACHE_DIR", &self.app.cache_engine.cache_dir);
        self.command
            .env("MOON_PROJECT_ID", self.project.id.as_str());
        self.command.env("MOON_PROJECT_ROOT", &self.project.root);
        self.command
            .env("MOON_PROJECT_SOURCE", self.project.source.as_str());
        self.command.env("MOON_TASK_ID", self.task.id.as_str());
        self.command.env("MOON_TASK_HASH", hash);
        self.command.env("MOON_TARGET", self.task.target.as_str());
        self.command
            .env("MOON_WORKSPACE_ROOT", &self.app.workspace_root);
        self.command.env("MOON_WORKING_DIR", &self.app.working_dir);
        self.command.env(
            "MOON_PROJECT_SNAPSHOT",
            self.app
                .cache_engine
                .state
                .get_project_snapshot_path(&self.project.id),
        );
    }

    #[instrument(skip_all)]
    fn inject_shell(&mut self) {
        if self.task.options.shell == Some(true) {
            // Process command set's a shell by default!

            #[cfg(unix)]
            if let Some(shell) = &self.task.options.unix_shell {
                use moon_config::TaskUnixShell;

                self.command.with_shell(match shell {
                    TaskUnixShell::Bash => Shell::new(ShellType::Bash),
                    TaskUnixShell::Elvish => Shell::new(ShellType::Elvish),
                    TaskUnixShell::Fish => Shell::new(ShellType::Fish),
                    TaskUnixShell::Ion => Shell::new(ShellType::Ion),
                    TaskUnixShell::Murex => Shell::new(ShellType::Murex),
                    TaskUnixShell::Nu => Shell::new(ShellType::Nu),
                    TaskUnixShell::Pwsh => Shell::new(ShellType::Pwsh),
                    TaskUnixShell::Xonsh => Shell::new(ShellType::Xonsh),
                    TaskUnixShell::Zsh => Shell::new(ShellType::Zsh),
                });
            }

            #[cfg(windows)]
            if let Some(shell) = &self.task.options.windows_shell {
                use moon_config::TaskWindowsShell;

                self.command.with_shell(match shell {
                    TaskWindowsShell::Bash => Shell::new(ShellType::Bash),
                    TaskWindowsShell::Elvish => Shell::new(ShellType::Elvish),
                    TaskWindowsShell::Fish => Shell::new(ShellType::Fish),
                    TaskWindowsShell::Murex => Shell::new(ShellType::Murex),
                    TaskWindowsShell::Nu => Shell::new(ShellType::Nu),
                    TaskWindowsShell::Pwsh => Shell::new(ShellType::Pwsh),
                    TaskWindowsShell::Xonsh => Shell::new(ShellType::Xonsh),
                });
            }
        } else {
            self.command.without_shell();
        }
    }

    #[instrument(skip_all)]
    fn inherit_affected(&mut self, context: &ActionContext) -> miette::Result<()> {
        let Some(check_affected) = &self.task.options.affected_files else {
            return Ok(());
        };

        // Only get files when `--affected` is passed
        let mut abs_files = if context.affected.is_some() {
            self.task.get_affected_files(
                &self.app.workspace_root,
                &context.changed_files,
                &self.project.source,
            )?
        } else {
            Vec::with_capacity(0)
        };

        // If we have no files, use the task's inputs instead
        if abs_files.is_empty() && self.task.options.affected_pass_inputs {
            abs_files = self.task.get_input_files(&self.app.workspace_root)?;
        }

        abs_files.sort();

        // Convert to relative paths
        let rel_files = abs_files
            .into_iter()
            .filter_map(|abs_file| {
                if self.working_dir == self.app.workspace_root
                    || abs_file.starts_with(&self.project.root)
                {
                    abs_file.relative_to(self.working_dir).ok()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Set an environment variable
        if matches!(
            check_affected,
            TaskOptionAffectedFiles::Env | TaskOptionAffectedFiles::Enabled(true)
        ) {
            self.command.env(
                "MOON_AFFECTED_FILES",
                if rel_files.is_empty() {
                    ".".into()
                } else {
                    rel_files
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
            if rel_files.is_empty() {
                self.command.arg_if_missing(".");
            } else {
                let args = rel_files
                    .into_iter()
                    .map(|file| {
                        // Mimic relative from ("./")
                        let arg = format!("./{file}");

                        // Escape files with special characters
                        if arg.contains(['*', '$', '+', '[', ']']) {
                            format!("\"{arg}\"")
                        } else {
                            match &self.command.shell {
                                Some(shell) => shell.instance.quote(&arg),
                                None => arg,
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                self.command.args(args);
            }
        }

        Ok(())
    }

    fn inherit_config(&mut self) {
        // Terminal colors
        if self
            .app
            .workspace_config
            .pipeline
            .inherit_colors_for_piped_tasks
        {
            self.command.inherit_colors();
        }
    }
}
