use crate::run_state::{load_output_logs, save_output_logs, RunTargetState};
use crate::target_hash::TargetHasher;
use crate::{errors::RunnerError, inputs_collector};
use console::Term;
use miette::IntoDiagnostic;
use moon_action::{ActionNode, ActionStatus, Attempt};
use moon_action_context::{ActionContext, TargetState};
use moon_cache_item::CacheItem;
use moon_config::{TaskOptionAffectedFiles, TaskOutputStyle};
use moon_emitter::{Emitter, Event, EventFlow};
use moon_hash::ContentHasher;
use moon_logger::{debug, warn};
use moon_platform::PlatformManager;
use moon_platform_runtime::Runtime;
use moon_process::{args, output_to_error, output_to_string, Command, Output};
use moon_project::Project;
use moon_target::{TargetError, TargetScope};
use moon_task::Task;
use moon_terminal::{label_checkpoint, Checkpoint};
use moon_tool::get_proto_env_vars;
use moon_utils::{is_ci, is_test_env, path, time};
use moon_workspace::Workspace;
use rustc_hash::FxHashMap;
use starbase_styles::color;
use starbase_utils::glob;
use std::sync::Arc;
use tokio::{
    task,
    time::{sleep, Duration},
};

const LOG_TARGET: &str = "moon:runner";

pub enum HydrateFrom {
    LocalCache,
    PreviousOutput,
    RemoteCache,
}

pub struct Runner<'a> {
    pub cache: CacheItem<RunTargetState>,

    pub node: Arc<ActionNode>,

    emitter: &'a Emitter,

    project: &'a Project,

    stderr: Term,

    stdout: Term,

    task: &'a Task,

    workspace: &'a Workspace,
}

impl<'a> Runner<'a> {
    pub fn new(
        emitter: &'a Emitter,
        workspace: &'a Workspace,
        project: &'a Project,
        task: &'a Task,
    ) -> miette::Result<Runner<'a>> {
        let mut cache = workspace
            .cache_engine
            .cache_state::<RunTargetState>(task.get_cache_dir().join("lastRun.json"))?;

        if cache.data.target.is_empty() {
            cache.data.target = task.target.to_string();
        }

        Ok(Runner {
            cache,
            node: Arc::new(ActionNode::None),
            emitter,
            project,
            stderr: Term::buffered_stderr(),
            stdout: Term::buffered_stdout(),
            task,
            workspace,
        })
    }

    /// Cache outputs to the `.moon/cache/outputs` folder and to the cloud,
    /// so that subsequent builds are faster, and any local outputs
    /// can be hydrated easily.
    pub async fn archive_outputs(&self) -> miette::Result<()> {
        let hash = &self.cache.data.hash;

        if hash.is_empty() || !self.is_archivable()? {
            return Ok(());
        }

        // Check that outputs actually exist
        if !self.task.outputs.is_empty() && !self.has_outputs(false)? {
            return Err(RunnerError::MissingOutput(self.task.target.id.clone()).into());
        }

        // If so, then cache the archive
        if let EventFlow::Return(archive_path) = self
            .emitter
            .emit(Event::TargetOutputArchiving {
                hash,
                project: self.project,
                target: &self.task.target,
                task: self.task,
            })
            .await?
        {
            self.emitter
                .emit(Event::TargetOutputArchived {
                    archive_path: archive_path.into(),
                    hash,
                    project: self.project,
                    target: &self.task.target,
                    task: self.task,
                })
                .await?;
        }

        Ok(())
    }

    pub async fn hydrate(&self, from: HydrateFrom) -> miette::Result<ActionStatus> {
        // Only hydrate when the hash is different from the previous build,
        // as we can assume the outputs from the previous build still exist?
        if matches!(from, HydrateFrom::LocalCache) || matches!(from, HydrateFrom::RemoteCache) {
            self.hydrate_outputs().await?;
        }

        let mut comments = vec![match from {
            HydrateFrom::LocalCache => "cached",
            HydrateFrom::RemoteCache => "cached from remote",
            HydrateFrom::PreviousOutput => "cached from previous run",
        }];

        if self.should_print_short_hash() {
            comments.push(self.get_short_hash());
        }

        self.print_checkpoint(Checkpoint::RunPassed, &comments)?;
        self.print_cache_item()?;
        self.flush_output()?;

        Ok(if matches!(from, HydrateFrom::RemoteCache) {
            ActionStatus::CachedFromRemote
        } else {
            ActionStatus::Cached
        })
    }

    /// If we are cached (hash match), hydrate the project with the
    /// cached task outputs found in the hashed archive.
    pub async fn hydrate_outputs(&self) -> miette::Result<()> {
        let hash = &self.cache.data.hash;

        if hash.is_empty() {
            return Ok(());
        }

        // Hydrate outputs from the cache
        if let EventFlow::Return(archive_path) = self
            .emitter
            .emit(Event::TargetOutputHydrating {
                hash,
                project: self.project,
                target: &self.task.target,
                task: self.task,
            })
            .await?
        {
            self.emitter
                .emit(Event::TargetOutputHydrated {
                    archive_path: archive_path.into(),
                    hash,
                    project: self.project,
                    target: &self.task.target,
                    task: self.task,
                })
                .await?;
        }

        // Update the run state with the new hash
        self.cache.save()?;

        Ok(())
    }

    /// Create a hasher that is shared amongst all platforms.
    /// Primarily includes task information.
    pub async fn hash_common_target(
        &self,
        context: &ActionContext,
        hasher: &mut ContentHasher,
    ) -> miette::Result<()> {
        let vcs = &self.workspace.vcs;
        let task = &self.task;
        let project = &self.project;
        let workspace = &self.workspace;
        let mut hash = TargetHasher::new(task);

        hash.hash_project_deps(self.project.get_dependency_ids());
        hash.hash_task_deps(task, &context.target_states)?;

        if context.should_inherit_args(&task.target) {
            hash.hash_args(&context.passthrough_args);
        }

        hash.hash_inputs(
            inputs_collector::collect_and_hash_inputs(
                vcs,
                task,
                &project.root,
                &workspace.root,
                &workspace.config.hasher,
            )
            .await?,
        );

        hasher.hash_content(hash)?;

        Ok(())
    }

    pub async fn create_command(
        &self,
        context: &ActionContext,
        runtime: &Runtime,
    ) -> miette::Result<Command> {
        let workspace = &self.workspace;
        let project = &self.project;
        let task = &self.task;
        let working_dir = if task.options.run_from_workspace_root {
            &workspace.root
        } else {
            &project.root
        };

        debug!(
            target: LOG_TARGET,
            "Creating {} command (in working directory {})",
            color::label(&task.target),
            color::path(working_dir)
        );

        let mut command = PlatformManager::read()
            .get(task.platform)?
            .create_run_target_command(context, project, task, runtime, working_dir)
            .await?;

        command
            .cwd(working_dir)
            // We need to handle non-zero's manually
            .set_error_on_nonzero(false);

        self.create_env_vars(&mut command).await?;

        // Wrap in a shell
        if task.options.shell.is_none() || task.options.shell.is_some_and(|s| !s) {
            command.without_shell();
        }

        // Passthrough args
        if context.should_inherit_args(&self.task.target) {
            command.args(&context.passthrough_args);
        }

        // Terminal colors
        if self.workspace.config.runner.inherit_colors_for_piped_tasks {
            command.inherit_colors();
        }

        // Dependency specific args/env
        if let ActionNode::RunTask { args, env, .. } = &*self.node {
            command.args(args);
            command.envs(env.to_owned());
        }

        // Affected files (must be last args)
        if let Some(check_affected) = &self.task.options.affected_files {
            let mut affected_files = if context.affected_only {
                self.task
                    .get_affected_files(&context.touched_files, self.project.source.as_str())?
            } else {
                Vec::with_capacity(0)
            };

            affected_files.sort();

            if matches!(
                check_affected,
                TaskOptionAffectedFiles::Env | TaskOptionAffectedFiles::Enabled(true)
            ) {
                command.env(
                    "MOON_AFFECTED_FILES",
                    if affected_files.is_empty() {
                        ".".into()
                    } else {
                        affected_files
                            .iter()
                            .map(|f| f.as_str().to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    },
                );
            }

            if matches!(
                check_affected,
                TaskOptionAffectedFiles::Args | TaskOptionAffectedFiles::Enabled(true)
            ) {
                if affected_files.is_empty() {
                    command.arg_if_missing(".");
                } else {
                    // Mimic relative from ("./")
                    command.args(affected_files.iter().map(|f| format!("./{f}")));
                }
            }
        }

        Ok(command)
    }

    pub async fn create_env_vars(&self, command: &mut Command) -> miette::Result<()> {
        let mut env_vars = FxHashMap::default();

        env_vars.insert(
            "MOON_CACHE_DIR".to_owned(),
            path::to_string(&self.workspace.cache_engine.cache_dir)?,
        );
        env_vars.insert("MOON_PROJECT_ID".to_owned(), self.project.id.to_string());
        env_vars.insert(
            "MOON_PROJECT_ROOT".to_owned(),
            path::to_string(&self.project.root)?,
        );
        env_vars.insert(
            "MOON_PROJECT_SOURCE".to_owned(),
            self.project.source.to_string(),
        );
        env_vars.insert("MOON_TARGET".to_owned(), self.task.target.id.to_string());
        env_vars.insert(
            "MOON_WORKSPACE_ROOT".to_owned(),
            path::to_string(&self.workspace.root)?,
        );
        env_vars.insert(
            "MOON_WORKING_DIR".to_owned(),
            path::to_string(&self.workspace.working_dir)?,
        );
        env_vars.insert(
            "MOON_PROJECT_SNAPSHOT".to_owned(),
            path::to_string(
                self.workspace
                    .cache_engine
                    .states_dir
                    .join(self.project.get_cache_dir().join("snapshot.json")),
            )?,
        );

        command.envs(env_vars);
        command.envs(get_proto_env_vars());

        // Pin versions for each tool in the toolchain
        if let Some(bun_config) = &self.workspace.toolchain_config.bun {
            command.env_if_missing(
                "PROTO_BUN_VERSION",
                bun_config
                    .version
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "*".into()),
            );
        }

        if let Some(node_config) = &self.workspace.toolchain_config.node {
            command.env_if_missing(
                "PROTO_NODE_VERSION",
                node_config
                    .version
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "*".into()),
            );

            command.env_if_missing(
                "PROTO_NPM_VERSION",
                node_config
                    .npm
                    .version
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "*".into()),
            );

            if let Some(pnpm_config) = &node_config.pnpm {
                command.env_if_missing(
                    "PROTO_PNPM_VERSION",
                    pnpm_config
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "*".into()),
                );
            }

            if let Some(yarn_config) = &node_config.yarn {
                command.env_if_missing(
                    "PROTO_YARN_VERSION",
                    yarn_config
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "*".into()),
                );
            }
        }

        // Pin a version for all plugins so that global/system tasks work well
        for plugin in self.workspace.proto_config.plugins.keys() {
            command.env_if_missing(
                format!(
                    "PROTO_{}_VERSION",
                    plugin.as_str().to_uppercase().replace('-', "_")
                ),
                "*",
            );
        }

        Ok(())
    }

    pub fn flush_output(&self) -> miette::Result<()> {
        self.stdout.flush().into_diagnostic()?;
        self.stderr.flush().into_diagnostic()?;

        Ok(())
    }

    pub fn get_short_hash(&self) -> &str {
        if self.cache.data.hash.is_empty() {
            "" // Empty when cache is disabled
        } else {
            &self.cache.data.hash[0..8]
        }
    }

    pub fn has_outputs(&self, bypass_globs: bool) -> miette::Result<bool> {
        // If using globs, we have no way to truly determine if all outputs
        // exist on the current file system, so always hydrate...
        if bypass_globs && !self.task.output_globs.is_empty() {
            return Ok(false);
        }

        // Check paths first since they are literal
        for output in &self.task.output_files {
            if !output.to_path(&self.workspace.root).exists() {
                return Ok(false);
            }
        }

        // Check globs last, as they are costly
        if !self.task.output_globs.is_empty() {
            let outputs = glob::walk_files(&self.workspace.root, &self.task.output_globs)?;

            return Ok(!outputs.is_empty());
        }

        Ok(true)
    }

    /// Determine if the current task can be archived.
    pub fn is_archivable(&self) -> miette::Result<bool> {
        let task = self.task;

        if task.is_build_type() {
            return Ok(true);
        }

        for target in &self.workspace.config.runner.archivable_targets {
            let is_matching_task = task.target.task_id == target.task_id;

            match &target.scope {
                TargetScope::All => {
                    if is_matching_task {
                        return Ok(true);
                    }
                }
                TargetScope::Project(project_locator) => {
                    if let Some(owner_id) = task.target.get_project_id() {
                        if owner_id == project_locator && is_matching_task {
                            return Ok(true);
                        }
                    }
                }
                TargetScope::Tag(tag_id) => {
                    if self.project.config.tags.contains(tag_id) && is_matching_task {
                        return Ok(true);
                    }
                }
                TargetScope::Deps => return Err(TargetError::NoDepsInRunContext.into()),
                TargetScope::OwnSelf => return Err(TargetError::NoSelfInRunContext.into()),
            };
        }

        Ok(false)
    }

    /// Hash the target based on all current parameters and return early
    /// if this target hash has already been cached. Based on the state
    /// of the target and project, determine the hydration strategy as well.
    pub async fn is_cached(
        &mut self,
        context: &mut ActionContext,
        runtime: &Runtime,
    ) -> miette::Result<Option<HydrateFrom>> {
        let mut hasher = self
            .workspace
            .hash_engine
            .create_hasher(format!("Run {} target", self.task.target));

        self.hash_common_target(context, &mut hasher).await?;

        PlatformManager::read()
            .get(self.task.platform)?
            .hash_run_target(
                self.project,
                runtime,
                &mut hasher,
                &self.workspace.config.hasher,
            )
            .await?;

        let hash = hasher.generate_hash()?;

        debug!(
            target: LOG_TARGET,
            "Generated hash {} for target {}",
            color::hash(&hash),
            color::id(&self.task.target)
        );

        context.target_states.insert(
            self.task.target.clone(),
            TargetState::Completed(hash.clone()),
        );

        // Hash is the same as the previous build, so simply abort!
        // However, ensure the outputs also exist, otherwise we should hydrate
        if self.cache.data.exit_code == 0
            && self.cache.data.hash == hash
            && self.has_outputs(true)?
        {
            debug!(
                target: LOG_TARGET,
                "Cache hit for hash {}, reusing previous build",
                color::hash(&hash),
            );

            return Ok(Some(HydrateFrom::PreviousOutput));
        }

        self.cache.data.hash = hash.clone();

        // Refresh the hash manifest
        self.workspace.hash_engine.save_manifest(hasher)?;

        // Check if that hash exists in the cache
        if let EventFlow::Return(value) = self
            .emitter
            .emit(Event::TargetOutputCacheCheck {
                hash: &hash,
                target: &self.task.target,
            })
            .await?
        {
            match value.as_ref() {
                "local-cache" => {
                    debug!(
                        target: LOG_TARGET,
                        "Cache hit for hash {}, hydrating from local cache",
                        color::hash(&hash),
                    );

                    return Ok(Some(HydrateFrom::LocalCache));
                }
                "remote-cache" => {
                    debug!(
                        target: LOG_TARGET,
                        "Cache hit for hash {}, hydrating from remote cache",
                        color::hash(&hash),
                    );

                    return Ok(Some(HydrateFrom::RemoteCache));
                }
                _ => {}
            }
        }

        debug!(
            target: LOG_TARGET,
            "Cache miss for hash {}, continuing run",
            color::hash(&hash),
        );

        Ok(None)
    }

    /// Run the command as a child process and capture its output. If the process fails
    /// and `retry_count` is greater than 0, attempt the process again in case it passes.
    pub async fn run_command(
        &mut self,
        context: &ActionContext,
        command: &mut Command,
    ) -> miette::Result<Vec<Attempt>> {
        let attempt_total = self.task.options.retry_count + 1;
        let mut attempt_index = 1;
        let mut attempts = vec![];
        let primary_longest_width = context.primary_targets.iter().map(|t| t.id.len()).max();
        let is_primary = context.primary_targets.contains(&self.task.target);
        let is_real_ci = is_ci() && !is_test_env();
        let is_persistent = self.node.is_persistent() || self.task.is_persistent();
        let output;
        let error;

        // When a task is configured as local (no caching), or the interactive flag is passed,
        // we don't "capture" stdout/stderr (which breaks stdin) and let it stream natively.
        let is_interactive = (!self.task.options.cache && context.primary_targets.len() == 1)
            || self.node.is_interactive()
            || self.task.is_interactive();

        // When the primary target, always stream the output for a better developer experience.
        // However, transitive targets can opt into streaming as well.
        let should_stream_output = if let Some(output_style) = &self.task.options.output_style {
            matches!(output_style, TaskOutputStyle::Stream)
        } else {
            is_primary || is_real_ci
        };

        // Transitive targets may run concurrently, so differentiate them with a prefix.
        let stream_prefix = if is_real_ci || !is_primary || context.primary_targets.len() > 1 {
            Some(&self.task.target.id)
        } else {
            None
        };

        // For long-running process, log a message every 30 seconds to indicate it's still running
        let interval_target = self.task.target.clone();
        let interval_handle = task::spawn(async move {
            if is_persistent || is_interactive {
                return;
            }

            let mut secs = 0;

            loop {
                sleep(Duration::from_secs(30)).await;
                secs += 30;

                println!(
                    "{} {}",
                    label_checkpoint(&interval_target, Checkpoint::RunStart),
                    color::muted(format!("running for {}s", secs))
                );
            }
        });

        loop {
            let mut attempt = Attempt::new(attempt_index);

            self.print_target_label(Checkpoint::RunStart, &attempt, attempt_total)?;
            self.print_target_command(context, command)?;
            self.flush_output()?;

            let possible_output = if should_stream_output {
                if let Some(prefix) = stream_prefix {
                    command.set_prefix(prefix, primary_longest_width);
                }

                if is_interactive {
                    command.create_async().exec_stream_output().await
                } else {
                    command
                        .create_async()
                        .exec_stream_and_capture_output()
                        .await
                }
            } else {
                command.create_async().exec_capture_output().await
            };

            match possible_output {
                // zero and non-zero exit codes
                Ok(out) => {
                    attempt.done(if out.status.success() {
                        ActionStatus::Passed
                    } else {
                        ActionStatus::Failed
                    });

                    if should_stream_output {
                        self.handle_streamed_output(&mut attempt, attempt_total, &out)?;
                    } else {
                        self.handle_captured_output(&mut attempt, attempt_total, &out)?;
                    }

                    attempts.push(attempt);

                    if out.status.success() {
                        error = None;
                        output = out;

                        break;
                    } else if attempt_index >= attempt_total {
                        error = Some(RunnerError::RunFailed {
                            target: self.task.target.id.clone(),
                            query: format!(
                                "moon query hash {}",
                                if is_test_env() {
                                    "hash1234"
                                } else {
                                    self.get_short_hash()
                                }
                            ),
                            error: output_to_error(self.task.command.clone(), &out, false),
                        });
                        output = out;

                        break;
                    } else {
                        attempt_index += 1;

                        warn!(
                            target: LOG_TARGET,
                            "Target {} failed, running again with attempt {}",
                            color::label(&self.task.target),
                            attempt_index
                        );
                    }
                }
                // process itself failed
                Err(error) => {
                    attempt.done(ActionStatus::Failed);
                    attempts.push(attempt);

                    interval_handle.abort();

                    return Err(error);
                }
            }
        }

        interval_handle.abort();

        // Write the cache with the result and output
        self.cache.data.exit_code = output.status.code().unwrap_or(0);

        save_output_logs(
            self.cache.get_dir(),
            output_to_string(&output.stdout),
            output_to_string(&output.stderr),
        )?;

        if let Some(error) = error {
            return Err(error.into());
        }

        Ok(attempts)
    }

    pub async fn create_and_run_command(
        &mut self,
        context: &ActionContext,
        runtime: &Runtime,
    ) -> miette::Result<Vec<Attempt>> {
        let result = if self.task.is_no_op() {
            debug!(
                target: LOG_TARGET,
                "Target {} is a no operation, skipping",
                color::label(&self.task.target),
            );

            self.print_target_label(Checkpoint::RunPassed, &Attempt::new(0), 0)?;
            self.flush_output()?;

            Ok(vec![])
        } else {
            let mut command = self.create_command(context, runtime).await?;

            self.run_command(context, &mut command).await
        };

        self.cache.data.last_run_time = time::now_millis();
        self.cache.save()?;

        result
    }

    pub fn print_cache_item(&self) -> miette::Result<()> {
        let item = &self.cache;
        let (stdout, stderr) = load_output_logs(item.get_dir())?;

        self.print_output_with_style(&stdout, &stderr, item.data.exit_code != 0)?;

        Ok(())
    }

    pub fn print_checkpoint<T: AsRef<str>>(
        &self,
        checkpoint: Checkpoint,
        comments: &[T],
    ) -> miette::Result<()> {
        let label = label_checkpoint(&self.task.target, checkpoint);

        if comments.is_empty() {
            self.stdout.write_line(&label).into_diagnostic()?;
        } else {
            self.stdout
                .write_line(&format!(
                    "{} {}",
                    label,
                    color::muted(format!(
                        "({})",
                        comments
                            .iter()
                            .map(|c| c.as_ref())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                ))
                .into_diagnostic()?;
        }

        Ok(())
    }

    pub fn print_output_with_style(
        &self,
        stdout: &str,
        stderr: &str,
        failed: bool,
    ) -> miette::Result<()> {
        let print_stdout = || -> miette::Result<()> {
            if !stdout.is_empty() {
                self.stdout.write_line(stdout).into_diagnostic()?;
            }

            Ok(())
        };

        let print_stderr = || -> miette::Result<()> {
            if !stderr.is_empty() {
                self.stderr.write_line(stderr).into_diagnostic()?;
            }

            Ok(())
        };

        match self.task.options.output_style {
            // Only show output on failure
            Some(TaskOutputStyle::BufferOnlyFailure) => {
                if failed {
                    print_stdout()?;
                    print_stderr()?;
                }
            }
            // Only show the hash
            Some(TaskOutputStyle::Hash) => {
                let hash = &self.cache.data.hash;

                if !hash.is_empty() {
                    // Print to stderr so it can be captured
                    self.stderr.write_line(hash).into_diagnostic()?;
                }
            }
            // Show nothing
            Some(TaskOutputStyle::None) => {}
            // Show output on both success and failure
            _ => {
                print_stdout()?;
                print_stderr()?;
            }
        };

        Ok(())
    }

    pub fn print_target_command(
        &self,
        context: &ActionContext,
        command: &Command,
    ) -> miette::Result<()> {
        if !self.workspace.config.runner.log_running_command {
            return Ok(());
        }

        let task = &self.task;
        let mut args = vec![&task.command];
        args.extend(&task.args);

        if context.should_inherit_args(&task.target) {
            args.extend(&context.passthrough_args);
        }

        let command_line = args::join_args(args);

        let message = color::muted_light(command.inspect().format_command(
            &command_line,
            &self.workspace.root,
            Some(if task.options.run_from_workspace_root {
                &self.workspace.root
            } else {
                &self.project.root
            }),
        ));

        self.stdout.write_line(&message).into_diagnostic()?;

        Ok(())
    }

    pub fn print_target_label(
        &self,
        checkpoint: Checkpoint,
        attempt: &Attempt,
        attempt_total: u8,
    ) -> miette::Result<()> {
        let mut comments = vec![];

        if self.task.is_no_op() {
            comments.push("no op".to_owned());
        } else if attempt.index > 1 {
            comments.push(format!("{}/{}", attempt.index, attempt_total));
        }

        if let Some(duration) = attempt.duration {
            comments.push(time::elapsed(duration));
        }

        if self.should_print_short_hash() && attempt.finished_at.is_some() {
            comments.push(self.get_short_hash().to_owned());
        }

        self.print_checkpoint(checkpoint, &comments)?;

        Ok(())
    }

    // Print label *after* output has been captured, so parallel tasks
    // aren't intertwined and the labels align with the output.
    fn handle_captured_output(
        &self,
        attempt: &mut Attempt,
        attempt_total: u8,
        output: &Output,
    ) -> miette::Result<()> {
        self.print_target_label(
            if output.status.success() {
                Checkpoint::RunPassed
            } else {
                Checkpoint::RunFailed
            },
            attempt,
            attempt_total,
        )?;

        let stdout = output_to_string(&output.stdout);
        let stderr = output_to_string(&output.stderr);

        self.print_output_with_style(&stdout, &stderr, !output.status.success())?;
        self.flush_output()?;

        attempt.exit_code = output.status.code();
        attempt.stdout = Some(stdout);
        attempt.stderr = Some(stderr);

        Ok(())
    }

    // Only print the label when the process has failed,
    // as the actual output has already been streamed to the console.
    fn handle_streamed_output(
        &self,
        attempt: &mut Attempt,
        attempt_total: u8,
        output: &Output,
    ) -> miette::Result<()> {
        self.print_target_label(
            if output.status.success() {
                Checkpoint::RunPassed
            } else {
                Checkpoint::RunFailed
            },
            attempt,
            attempt_total,
        )?;

        self.flush_output()?;

        attempt.exit_code = output.status.code();
        attempt.stdout = Some(output_to_string(&output.stdout));
        attempt.stderr = Some(output_to_string(&output.stderr));

        Ok(())
    }

    fn should_print_short_hash(&self) -> bool {
        // Do not include the hash while testing, as the hash
        // constantly changes and breaks our local snapshots
        !is_test_env() && self.task.options.cache && !self.cache.data.hash.is_empty()
    }
}
