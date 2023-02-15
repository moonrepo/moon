use crate::errors::RunnerError;
use crate::target_hasher::TargetHasher;
use console::Term;
use moon_action::{ActionStatus, Attempt};
use moon_action_context::ActionContext;
use moon_cache::RunTargetState;
use moon_config::{HasherWalkStrategy, TaskOutputStyle};
use moon_emitter::{Emitter, Event, EventFlow};
use moon_error::MoonError;
use moon_hasher::{convert_paths_to_strings, HashSet};
use moon_logger::{color, debug, warn};
use moon_platform_runtime::Runtime;
use moon_project::Project;
use moon_target::{Target, TargetError, TargetProjectScope};
use moon_task::{Task, TaskError, TaskOptionAffectedFiles};
use moon_terminal::{label_checkpoint, Checkpoint};
use moon_utils::{
    glob, is_ci, is_test_env, path,
    process::{self, format_running_command, output_to_string, Command, Output},
    time,
};
use moon_workspace::Workspace;
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::path::PathBuf;

const LOG_TARGET: &str = "moon:runner";

pub enum HydrateFrom {
    LocalCache,
    PreviousOutput,
    RemoteCache,
}

pub struct Runner<'a> {
    pub cache: RunTargetState,

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
    ) -> Result<Runner<'a>, MoonError> {
        Ok(Runner {
            cache: workspace.cache.cache_run_target_state(&task.target)?,
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
    pub async fn archive_outputs(&self) -> Result<(), RunnerError> {
        let hash = &self.cache.hash;

        if hash.is_empty() || !self.is_archivable()? {
            return Ok(());
        }

        // Check that outputs actually exist
        // We don't check globs here as it would required walking the file system
        if !self.task.outputs.is_empty() {
            for output in &self.task.output_paths {
                if !output.exists() {
                    return Err(RunnerError::Task(TaskError::MissingOutput(
                        self.task.target.id.clone(),
                        path::to_string(output.strip_prefix(&self.project.root).unwrap())?,
                    )));
                }
            }
        }

        // If so, then cache the archive
        if let EventFlow::Return(archive_path) = self
            .emitter
            .emit(Event::TargetOutputArchiving {
                cache: &self.cache,
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

    pub async fn hydrate(&self, from: HydrateFrom) -> Result<ActionStatus, RunnerError> {
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
    pub async fn hydrate_outputs(&self) -> Result<(), RunnerError> {
        let hash = &self.cache.hash;

        if hash.is_empty() {
            return Ok(());
        }

        // Hydrate outputs from the cache
        if let EventFlow::Return(archive_path) = self
            .emitter
            .emit(Event::TargetOutputHydrating {
                cache: &self.cache,
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
        hashset: &mut HashSet,
    ) -> Result<(), RunnerError> {
        let vcs = &self.workspace.vcs;
        let task = &self.task;
        let project = &self.project;
        let workspace = &self.workspace;
        let globset = task.create_globset()?;
        let mut hasher = TargetHasher::new();
        let mut files_to_hash = vec![];

        hasher.hash_project_deps(self.project.get_dependency_ids());
        hasher.hash_task(task);
        hasher.hash_task_deps(task, &context.target_hashes);

        if context.should_inherit_args(&task.target) {
            hasher.hash_args(&context.passthrough_args);
        }

        // For inputs, hash them with the vcs layer first
        if !task.input_paths.is_empty() {
            let files = convert_paths_to_strings(&task.input_paths, &workspace.root)?;

            files_to_hash.extend(files);
        }

        if !task.input_globs.is_empty() {
            let use_globs = self.project.root == self.workspace.root
                || matches!(
                    workspace.config.hasher.walk_strategy,
                    HasherWalkStrategy::Glob
                );

            // Walk the file system using globs
            if use_globs {
                let globbed_files = glob::walk(
                    &workspace.root,
                    &task
                        .input_globs
                        .iter()
                        .map(|g| {
                            PathBuf::from(g)
                                .strip_prefix(&workspace.root)
                                .unwrap()
                                .to_string_lossy()
                                .to_string()
                        })
                        .collect::<Vec<_>>(),
                )?;

                let files = convert_paths_to_strings(
                    &FxHashSet::from_iter(globbed_files),
                    &workspace.root,
                )?;

                files_to_hash.extend(files);

                // Walk the file system using the VCS
            } else {
                let mut hashed_file_tree = vcs.get_file_tree_hashes(&project.source).await?;

                // Input globs are absolute paths, so we must do the same
                hashed_file_tree
                    .retain(|k, _| globset.matches(workspace.root.join(k)).unwrap_or(false));

                hasher.hash_inputs(hashed_file_tree);
            }
        }

        // Include local file changes so that development builds work.
        // Also run this LAST as it should take highest precedence!
        let local_files = vcs.get_touched_files().await?;

        if !local_files.all.is_empty() {
            // Only hash files that are within the task's inputs
            let files = local_files
                .all
                .into_iter()
                .filter(|f| globset.matches(workspace.root.join(f)).unwrap_or(false))
                .collect::<Vec<String>>();

            files_to_hash.extend(files);
        }

        if !files_to_hash.is_empty() {
            hasher.hash_inputs(vcs.get_file_hashes(&files_to_hash, true).await?);
        }

        hashset.hash(hasher);

        Ok(())
    }

    pub async fn create_command(
        &self,
        context: &ActionContext,
        runtime: &Runtime,
    ) -> Result<Command, RunnerError> {
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
            color::target(&task.target),
            color::path(working_dir)
        );

        let mut command = self
            .workspace
            .platforms
            .get(task.platform)?
            .create_run_target_command(context, project, task, runtime, working_dir)
            .await?;

        command
            .cwd(working_dir)
            .envs(self.create_env_vars().await?)
            // We need to handle non-zero's manually
            .no_error_on_failure();

        // Passthrough args
        if context.should_inherit_args(&self.task.target) {
            command.args(&context.passthrough_args);
        }

        // Terminal colors
        if self.workspace.config.runner.inherit_colors_for_piped_tasks {
            command.inherit_colors();
        }

        // Affected files (must be last args)
        if let Some(check_affected) = &self.task.options.affected_files {
            let mut affected_files = if context.affected_only {
                self.task
                    .get_affected_files(&context.touched_files, &self.project.root)?
            } else {
                Vec::with_capacity(0)
            };

            affected_files.sort();

            if matches!(
                check_affected,
                TaskOptionAffectedFiles::Env | TaskOptionAffectedFiles::Both
            ) {
                command.env(
                    "MOON_AFFECTED_FILES",
                    if affected_files.is_empty() {
                        ".".into()
                    } else {
                        affected_files
                            .iter()
                            .map(|f| f.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(",")
                    },
                );
            }

            if matches!(
                check_affected,
                TaskOptionAffectedFiles::Args | TaskOptionAffectedFiles::Both
            ) {
                if affected_files.is_empty() {
                    command.arg_if_missing(".");
                } else {
                    command.args(affected_files);
                }
            }
        }

        Ok(command)
    }

    pub async fn create_env_vars(&self) -> Result<FxHashMap<String, String>, MoonError> {
        let mut env_vars = FxHashMap::default();

        env_vars.insert(
            "MOON_CACHE_DIR".to_owned(),
            path::to_string(&self.workspace.cache.dir)?,
        );
        env_vars.insert("MOON_PROJECT_ID".to_owned(), self.project.id.clone());
        env_vars.insert(
            "MOON_PROJECT_ROOT".to_owned(),
            path::to_string(&self.project.root)?,
        );
        env_vars.insert(
            "MOON_PROJECT_SOURCE".to_owned(),
            self.project.source.clone(),
        );
        env_vars.insert("MOON_TARGET".to_owned(), self.task.target.id.clone());
        env_vars.insert(
            "MOON_TOOLCHAIN_DIR".to_owned(),
            env::var("PROTO_ROOT").unwrap(),
        );
        env_vars.insert(
            "MOON_WORKSPACE_ROOT".to_owned(),
            path::to_string(&self.workspace.root)?,
        );
        env_vars.insert(
            "MOON_WORKING_DIR".to_owned(),
            path::to_string(&self.workspace.working_dir)?,
        );

        // Store runtime data on the file system so that downstream commands can utilize it
        let runfile = self
            .workspace
            .cache
            .create_runfile(&self.project.id, self.project)?;

        env_vars.insert(
            "MOON_PROJECT_RUNFILE".to_owned(),
            path::to_string(runfile.path)?,
        );

        Ok(env_vars)
    }

    pub fn get_short_hash(&self) -> &str {
        if self.cache.hash.is_empty() {
            "" // Empty when cache is disabled
        } else {
            &self.cache.hash[0..8]
        }
    }

    pub fn flush_output(&self) -> Result<(), MoonError> {
        self.stdout.flush()?;
        self.stderr.flush()?;

        Ok(())
    }

    /// Determine if the current task can be archived.
    pub fn is_archivable(&self) -> Result<bool, TargetError> {
        let task = self.task;

        if task.is_build_type() {
            return Ok(true);
        }

        for target in &self.workspace.config.runner.archivable_targets {
            let target = Target::parse(target)?;

            match &target.project {
                TargetProjectScope::All => {
                    if task.target.task_id == target.task_id {
                        return Ok(true);
                    }
                }
                TargetProjectScope::Id(project_id) => {
                    if let Some(owner_id) = &task.target.project_id {
                        if owner_id == project_id && task.target.task_id == target.task_id {
                            return Ok(true);
                        }
                    }
                }
                TargetProjectScope::Deps => return Err(TargetError::NoProjectDepsInRunContext),
                TargetProjectScope::OwnSelf => return Err(TargetError::NoProjectSelfInRunContext),
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
    ) -> Result<Option<HydrateFrom>, RunnerError> {
        let mut hashset = HashSet::default();

        self.hash_common_target(context, &mut hashset).await?;

        self.workspace
            .platforms
            .get(self.task.platform)?
            .hash_run_target(
                self.project,
                runtime,
                &mut hashset,
                &self.workspace.config.hasher,
            )
            .await?;

        let hash = hashset.generate();

        debug!(
            target: LOG_TARGET,
            "Generated hash {} for target {}",
            color::hash(&hash),
            color::id(&self.task.target)
        );

        context
            .target_hashes
            .insert(self.task.target.id.clone(), hash.clone());

        // Hash is the same as the previous build, so simply abort!
        // However, ensure the outputs also exist, otherwise we should hydrate.
        if self.cache.hash == hash && self.has_outputs() {
            debug!(
                target: LOG_TARGET,
                "Cache hit for hash {}, reusing previous build",
                color::hash(&hash),
            );

            return Ok(Some(HydrateFrom::PreviousOutput));
        }

        self.cache.hash = hash.clone();

        // Refresh the hash manifest
        self.workspace.cache.create_hash_manifest(&hash, &hashset)?;

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

    /// Return true if this target is a no-op.
    pub fn is_no_op(&self) -> bool {
        self.task.is_no_op()
    }

    /// Verify that all task outputs exist for the current target.
    pub fn has_outputs(&self) -> bool {
        self.task.output_paths.iter().all(|p| p.exists())
    }

    /// Run the command as a child process and capture its output. If the process fails
    /// and `retry_count` is greater than 0, attempt the process again in case it passes.
    pub async fn run_command(
        &mut self,
        context: &ActionContext,
        command: &mut Command,
    ) -> Result<Vec<Attempt>, RunnerError> {
        let attempt_total = self.task.options.retry_count + 1;
        let mut attempt_index = 1;
        let mut attempts = vec![];
        let primary_longest_width = context.primary_targets.iter().map(|t| t.id.len()).max();
        let is_primary = context.primary_targets.contains(&self.task.target);
        let is_real_ci = is_ci() && !is_test_env();
        let output;

        // When a task is configured as local (no caching), or the interactive flag is passed,
        // we don't "capture" stdout/stderr (which breaks stdin) and let it stream natively.
        let is_interactive =
            (!self.task.options.cache && context.primary_targets.len() == 1) || context.interactive;

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

        loop {
            let mut attempt = Attempt::new(attempt_index);

            self.print_target_label(Checkpoint::RunStart, &attempt, attempt_total)?;
            self.print_target_command(context)?;
            self.flush_output()?;

            let possible_output = if should_stream_output {
                if let Some(prefix) = stream_prefix {
                    command.set_prefix(prefix, primary_longest_width);
                }

                if is_interactive {
                    command.exec_stream_output().await
                } else {
                    command.exec_stream_and_capture_output().await
                }
            } else {
                command.exec_capture_output().await
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
                        self.handle_streamed_output(&attempt, attempt_total, &out)?;
                    } else {
                        self.handle_captured_output(&attempt, attempt_total, &out)?;
                    }

                    attempts.push(attempt);

                    if out.status.success() {
                        output = out;
                        break;
                    } else if attempt_index >= attempt_total {
                        return Err(RunnerError::Moon(command.output_to_error(&out, false)));
                    } else {
                        attempt_index += 1;

                        warn!(
                            target: LOG_TARGET,
                            "Target {} failed, running again with attempt {}",
                            color::target(&self.task.target),
                            attempt_index
                        );
                    }
                }
                // process itself failed
                Err(error) => {
                    attempt.done(ActionStatus::Failed);
                    attempts.push(attempt);

                    return Err(RunnerError::Moon(error));
                }
            }
        }

        // Write the cache with the result and output
        self.cache.exit_code = output.status.code().unwrap_or(0);
        self.cache.last_run_time = time::now_millis();
        self.cache.save()?;
        self.cache.save_output_logs(
            output_to_string(&output.stdout),
            output_to_string(&output.stderr),
        )?;

        Ok(attempts)
    }

    pub fn print_cache_item(&self) -> Result<(), MoonError> {
        let item = &self.cache;
        let (stdout, stderr) = item.load_output_logs()?;

        self.print_output_with_style(&stdout, &stderr, item.exit_code != 0)?;

        Ok(())
    }

    pub fn print_checkpoint<T: AsRef<str>>(
        &self,
        checkpoint: Checkpoint,
        comments: &[T],
    ) -> Result<(), MoonError> {
        let label = label_checkpoint(&self.task.target, checkpoint);

        if comments.is_empty() {
            self.stdout.write_line(&label)?;
        } else {
            self.stdout.write_line(&format!(
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
            ))?;
        }

        Ok(())
    }

    pub fn print_output_with_style(
        &self,
        stdout: &str,
        stderr: &str,
        failed: bool,
    ) -> Result<(), MoonError> {
        let print_stdout = || -> Result<(), MoonError> {
            if !stdout.is_empty() {
                self.stdout.write_line(stdout)?;
            }

            Ok(())
        };

        let print_stderr = || -> Result<(), MoonError> {
            if !stderr.is_empty() {
                self.stderr.write_line(stderr)?;
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
                let hash = &self.cache.hash;

                if !hash.is_empty() {
                    // Print to stderr so it can be captured
                    self.stderr.write_line(hash)?;
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

    pub fn print_target_command(&self, context: &ActionContext) -> Result<(), MoonError> {
        if !self.workspace.config.runner.log_running_command {
            return Ok(());
        }

        let task = &self.task;
        let mut args = vec![];
        args.extend(&task.args);

        if context.should_inherit_args(&task.target) {
            args.extend(&context.passthrough_args);
        }

        let command_line = if args.is_empty() {
            task.command.clone()
        } else {
            format!("{} {}", task.command, process::join_args(args))
        };

        let message = format_running_command(
            &command_line,
            Some(if task.options.run_from_workspace_root {
                &self.workspace.root
            } else {
                &self.project.root
            }),
            Some(&self.workspace.root),
        );

        self.stdout.write_line(&message)?;

        Ok(())
    }

    pub fn print_target_label(
        &self,
        checkpoint: Checkpoint,
        attempt: &Attempt,
        attempt_total: u8,
    ) -> Result<(), MoonError> {
        let mut comments = vec![];

        if attempt.index > 1 {
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
        attempt: &Attempt,
        attempt_total: u8,
        output: &Output,
    ) -> Result<(), MoonError> {
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

        Ok(())
    }

    // Only print the label when the process has failed,
    // as the actual output has already been streamed to the console.
    fn handle_streamed_output(
        &self,
        attempt: &Attempt,
        attempt_total: u8,
        output: &Output,
    ) -> Result<(), MoonError> {
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

        Ok(())
    }

    fn should_print_short_hash(&self) -> bool {
        // Do not include the hash while testing, as the hash
        // constantly changes and breaks our local snapshots
        !is_test_env() && self.task.options.cache && !self.cache.hash.is_empty()
    }
}
