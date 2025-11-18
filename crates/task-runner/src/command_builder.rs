use moon_action::ActionNode;
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::path::PathExt;
use moon_config::TaskOptionAffectedFiles;
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::{
    Extend, ExtendCommandInput, ExtendCommandOutput, ExtendTaskCommandInput, ExtendTaskScriptInput,
    ExtendTaskScriptOutput,
};
use moon_process::{Command, Shell, ShellType};
use moon_project::Project;
use moon_task::Task;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, trace};

#[derive(Default)]
struct CommandParams {
    exe: String,
    args: VecDeque<String>,
    env: FxHashMap<String, String>,
    env_remove: Vec<String>,
    paths: VecDeque<PathBuf>,
}

impl CommandParams {
    fn apply_outputs(&mut self, outputs: Vec<ExtendCommandOutput>) {
        for output in outputs {
            if let Some(new_bin) = output.command {
                self.exe = new_bin;
            }

            if let Some(new_args) = output.args {
                self.extend_args(new_args);
            }

            self.extend_env(output.env, output.env_remove);
            self.extend_paths(output.paths);
        }
    }

    fn apply_script_outputs(&mut self, outputs: Vec<ExtendTaskScriptOutput>) {
        for output in outputs {
            if let Some(new_script) = output.script {
                self.exe = new_script;
            }

            self.extend_env(output.env, output.env_remove);
            self.extend_paths(output.paths);
        }
    }

    fn extend_args(&mut self, args: Extend<Vec<String>>) {
        match args {
            Extend::Empty => {
                self.args.clear();
            }
            Extend::Append(next) => {
                self.args.extend(next);
            }
            Extend::Prepend(next) => {
                for arg in next.into_iter().rev() {
                    self.args.push_front(arg);
                }
            }
            Extend::Replace(next) => {
                self.args.clear();
                self.args.extend(next);
            }
        }
    }

    fn extend_env(&mut self, env: FxHashMap<String, String>, env_remove: Vec<String>) {
        self.env.extend(env);
        self.env_remove.extend(env_remove);
    }

    fn extend_paths(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        // Normalize separators since WASM always uses forward slashes
        #[cfg(windows)]
        let paths = paths.into_iter().map(|path| {
            PathBuf::from(moon_common::path::normalize_separators(
                path.to_string_lossy(),
            ))
        });

        for path in paths.into_iter().rev() {
            self.paths.push_front(path);
        }
    }
}

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
        self.inherit_proto().await?;

        // Must be last!
        self.command.inherit_path()?;

        Ok(self.command)
    }

    async fn build_command(&mut self) -> miette::Result<Command> {
        let project = self.project;
        let task = self.task;
        let toolchain_ids = project.get_enabled_toolchains_for_task(task);

        let mut escape_args = true;
        let mut params = CommandParams::default();
        params.args.extend(task.args.clone());
        params.env.extend(task.env.clone());

        params.apply_outputs(
            self.app
                .toolchain_registry
                .extend_command_many(toolchain_ids.clone(), |registry, toolchain| {
                    ExtendCommandInput {
                        context: registry.create_context(),
                        command: params.exe.clone(),
                        args: params.args.clone().into_iter().collect(),
                        current_dir: registry.to_virtual_path(&project.root),
                        toolchain_config: registry
                            .create_merged_config(&toolchain.id, &project.config),
                        ..Default::default()
                    }
                })
                .await?,
        );

        params.apply_outputs(
            self.app
                .extension_registry
                .extend_command_all(|registry, extension| ExtendCommandInput {
                    context: registry.create_context(),
                    command: params.exe.clone(),
                    args: params.args.clone().into_iter().collect(),
                    current_dir: registry.to_virtual_path(&project.root),
                    extension_config: registry.create_config(&extension.id),
                    ..Default::default()
                })
                .await?,
        );

        match &task.script {
            Some(script) => {
                // Scripts should be used as-is
                escape_args = false;

                params.exe = script.into();
                params.args.clear();

                params.apply_script_outputs(
                    self.app
                        .toolchain_registry
                        .extend_task_script_many(toolchain_ids, |registry, toolchain| {
                            ExtendTaskScriptInput {
                                context: registry.create_context(),
                                script: params.exe.clone(),
                                project: project.to_fragment(),
                                task: task.to_fragment(),
                                toolchain_config: registry
                                    .create_merged_config(&toolchain.id, &project.config),
                                ..Default::default()
                            }
                        })
                        .await?,
                );

                params.apply_script_outputs(
                    self.app
                        .extension_registry
                        .extend_task_script_all(|registry, extension| ExtendTaskScriptInput {
                            context: registry.create_context(),
                            script: params.exe.clone(),
                            project: project.to_fragment(),
                            task: task.to_fragment(),
                            extension_config: registry.create_config(&extension.id),
                            ..Default::default()
                        })
                        .await?,
                );
            }
            None => {
                params.exe = task.command.clone();

                params.apply_outputs(
                    self.app
                        .toolchain_registry
                        .extend_task_command_many(toolchain_ids, |registry, toolchain| {
                            ExtendTaskCommandInput {
                                context: registry.create_context(),
                                command: params.exe.clone(),
                                args: params.args.clone().into_iter().collect(),
                                project: project.to_fragment(),
                                task: task.to_fragment(),
                                toolchain_config: registry
                                    .create_merged_config(&toolchain.id, &project.config),
                                ..Default::default()
                            }
                        })
                        .await?,
                );

                params.apply_outputs(
                    self.app
                        .extension_registry
                        .extend_task_command_all(|registry, extension| ExtendTaskCommandInput {
                            context: registry.create_context(),
                            command: params.exe.clone(),
                            args: params.args.clone().into_iter().collect(),
                            project: project.to_fragment(),
                            task: task.to_fragment(),
                            extension_config: registry.create_config(&extension.id),
                            ..Default::default()
                        })
                        .await?,
                );
            }
        };

        let mut command = Command::new(params.exe);
        command.escape_args = escape_args;
        command.args(params.args);
        command.prepend_paths(params.paths);
        command.envs_if_not_global(params.env);

        for key in params.env_remove {
            command.env_remove(key);
        }

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

    async fn inherit_proto(&mut self) -> miette::Result<()> {
        let toolchain_registry = &self.app.toolchain_registry;
        let toolchain_ids = self.project.get_enabled_toolchains_for_task(self.task);
        let mut augments = toolchain_registry.create_command_augments(Some(&self.project.config));

        // Only include paths for toolchains that this task explicitly needs,
        // but keep environment variables and other parameters
        augments.iter_mut().for_each(|(id, augment)| {
            augment.add_path = toolchain_ids.contains(&id);
        });

        toolchain_registry
            .augment_command(&mut self.command, self.env_bag, augments)
            .await?;

        Ok(())
    }
}
