use miette::IntoDiagnostic;
use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::is_ci;
use moon_common::path::PathExt;
use moon_config::{Input, TaskOptionAffectedFilesPattern};
use moon_env_var::{DotEnv, GlobalEnvBag};
use moon_process::{Command, Shell, ShellType};
use moon_process_augment::AugmentedCommand;
use moon_project::Project;
use moon_task::Task;
use rustc_hash::FxHashMap;
use std::env;
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
    command: AugmentedCommand<'task>,
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
            command: AugmentedCommand::new(app, GlobalEnvBag::instance(), "noop"),
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

        self.command = AugmentedCommand::from_task(self.app, self.env_bag, self.task);
        self.command
            .inherit_from_plugins(Some(self.project), Some(self.task))
            .await?;

        // We need to handle non-zero exit code's manually
        self.command.cwd(self.working_dir);
        self.command.set_error_on_nonzero(false);

        // Order is important!
        self.inject_args(context);
        self.inject_env(hash)?;
        self.inject_shell();
        self.inherit_affected(context)?;
        self.inherit_config();

        // Must be last!
        self.command.inherit_proto();

        Ok(self.command.augment())
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
    fn inject_env(&mut self, hash: &str) -> miette::Result<()> {
        let task = self.task;
        let mut moon_env = FxHashMap::<String, Option<String>>::default();

        // Inherit task dependent variables
        if let ActionNode::RunTask(inner) = &self.node
            && !inner.env.is_empty()
        {
            trace!(
                task_target = self.task.target.as_str(),
                env = ?inner.env,
                "Inheriting env from dependent task"
            );

            moon_env.extend(inner.env.clone());
        }

        // Inherit moon variables
        let make_path = |path: &Path| Some(path.to_string_lossy().to_string());

        moon_env.extend(FxHashMap::from_iter([
            (
                "MOON_CACHE_DIR".into(),
                make_path(&self.app.cache_engine.cache_dir),
            ),
            ("MOON_PROJECT_ID".into(), Some(self.project.id.to_string())),
            ("MOON_PROJECT_ROOT".into(), make_path(&self.project.root)),
            (
                "MOON_PROJECT_SOURCE".into(),
                Some(self.project.source.to_string()),
            ),
            (
                "MOON_PROJECT_SNAPSHOT".into(),
                make_path(
                    &self
                        .app
                        .cache_engine
                        .state
                        .get_project_snapshot_path(&self.project.id),
                ),
            ),
            ("MOON_TASK_ID".into(), Some(task.id.to_string())),
            ("MOON_TASK_HASH".into(), Some(hash.to_string())),
            ("MOON_TARGET".into(), Some(task.target.to_string())),
            (
                "MOON_WORKSPACE_ROOT".into(),
                make_path(&self.app.workspace_root),
            ),
            ("MOON_WORKING_DIR".into(), make_path(&self.app.working_dir)),
            ("PWD".into(), make_path(self.working_dir)),
        ]));

        // Load variables from .env files
        if let Some(env_files) = &self.task.options.env_files {
            let env_paths = env_files
                .iter()
                .filter_map(|input| match input {
                    Input::File(file) => Some(
                        file.to_workspace_relative(self.project.source.as_str())
                            .to_path(&self.app.workspace_root),
                    ),
                    _ => None,
                })
                .collect::<Vec<_>>();

            trace!(
                task_target = task.target.as_str(),
                env_files = ?env_paths,
                "Loading environment variables from .env files",
            );

            let mut dot_env = FxHashMap::default();
            let ci = is_ci();

            for env_path in env_paths {
                // The file may not have been committed, so avoid crashing
                if env_path.exists() {
                    // Skip local only env files
                    if ci
                        && env_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.ends_with(".local"))
                    {
                        trace!(
                            task_target = task.target.as_str(),
                            env_file = ?env_path,
                            "Skipping .env file because we're in CI and it's local only",
                        );

                        continue;
                    }

                    trace!(
                        task_target = task.target.as_str(),
                        env_file = ?env_path,
                        "Loading .env file",
                    );

                    // Overwrite previous values
                    dot_env.extend(
                        DotEnv::default()
                            .with_global_vars(self.env_bag)
                            // Can reference vars from previous dotenv files
                            .with_local_vars(&dot_env)
                            // Can reference task vars, but they take higher
                            // precedence than dotenv vars
                            .with_local_vars(&self.task.env)
                            // Can also reference moon vars
                            .with_local_vars(&moon_env)
                            .load_file(&env_path)?,
                    );
                } else {
                    trace!(
                        task_target = task.target.as_str(),
                        env_file = ?env_path,
                        "Skipping .env file because it doesn't exist",
                    );
                }
            }

            for (key, value) in dot_env {
                if
                // Don't override task-level variables
                !self.command.contains_env(&key)
                    // Don't override system variables
                    && !self.env_bag.has(&key)
                {
                    self.command.env_opt(key, value);
                }
            }
        }

        self.command.envs_opt(moon_env);

        Ok(())
    }

    #[instrument(skip_all)]
    fn inject_shell(&mut self) {
        if self.task.options.shell == Some(true) {
            #[cfg(unix)]
            {
                use moon_config::TaskUnixShell;

                self.command.set_shell(match self.task.options.unix_shell {
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
            {
                use moon_config::TaskWindowsShell;

                self.command
                    .set_shell(match self.task.options.windows_shell {
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
            self.command.no_shell();
        }
    }

    #[instrument(skip_all)]
    fn inherit_affected(&mut self, context: &ActionContext) -> miette::Result<()> {
        let Some(affected_options) = &self.task.options.affected_files else {
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
        if abs_files.is_empty() && affected_options.pass_inputs_when_no_match {
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
            affected_options.pass,
            TaskOptionAffectedFilesPattern::Env | TaskOptionAffectedFilesPattern::Enabled(true)
        ) {
            self.command.env(
                "MOON_AFFECTED_FILES",
                if rel_files.is_empty() {
                    ".".into()
                } else {
                    env::join_paths(
                        rel_files
                            .iter()
                            .map(|file| file.as_str())
                            .collect::<Vec<_>>(),
                    )
                    .into_diagnostic()?
                },
            );
        }

        // Pass an argument
        if matches!(
            affected_options.pass,
            TaskOptionAffectedFilesPattern::Args | TaskOptionAffectedFilesPattern::Enabled(true)
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
