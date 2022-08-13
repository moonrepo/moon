use crate::action::Attempt;
use crate::context::ActionContext;
use crate::errors::ActionError;
use moon_cache::{CacheItem, RunTargetState};
use moon_config::TaskOutputStyle;
use moon_error::MoonError;
use moon_hasher::{convert_paths_to_strings, to_hash, Hasher, TargetHasher};
use moon_logger::{color, debug, warn};
use moon_project::Project;
use moon_task::Task;
use moon_terminal::{label_checkpoint, Checkpoint};
use moon_utils::{
    fs, is_ci, is_test_env, path,
    process::{self, output_to_string, Command, Output},
    time,
};
use moon_workspace::Workspace;
use serde::Serialize;
use std::collections::HashMap;

const LOG_TARGET: &str = "moon:action:run-target";

pub enum HydrateFrom {
    LocalCache,
    PreviousOutput,
}

pub struct TargetRunner<'a> {
    pub cache: CacheItem<RunTargetState>,

    project: &'a Project,

    task: &'a Task,

    workspace: &'a Workspace,
}

impl<'a> TargetRunner<'a> {
    pub async fn new(
        workspace: &'a Workspace,
        project: &'a Project,
        task: &'a Task,
    ) -> Result<TargetRunner<'a>, MoonError> {
        Ok(TargetRunner {
            cache: workspace.cache.cache_run_target_state(&task.target).await?,
            project,
            task,
            workspace,
        })
    }

    /// Cache outputs to the `.moon/cache/out` folder and to the cloud,
    /// so that subsequent builds are faster, and any local outputs
    /// can be rehydrated easily.
    pub async fn cache_outputs(&self) -> Result<(), ActionError> {
        let hash = &self.cache.item.hash;

        if !hash.is_empty() && !self.task.outputs.is_empty() {
            self.workspace
                .cache
                .create_hash_archive(hash, &self.project.root, &self.task.outputs)
                .await?;
        }

        Ok(())
    }

    /// If we are cached (hash match), hydrate the project with the
    /// cached task outputs found in the hashed archive.
    pub async fn hydrate_outputs(&self) -> Result<(), ActionError> {
        let hash = &self.cache.item.hash;

        if hash.is_empty() {
            return Ok(());
        }

        // Remove previous outputs so we avoid stale artifacts
        for output in &self.task.output_paths {
            fs::remove(output).await?;
        }

        // Hydrate outputs from the cache
        self.workspace
            .cache
            .hydrate_from_hash_archive(hash, &self.project.root)
            .await?;

        // Update the run state with the new hash
        self.cache.save().await?;

        Ok(())
    }

    /// Create a hasher that is shared amongst all platforms.
    /// Primarily includes task information.
    pub async fn create_common_hasher(
        &self,
        context: &ActionContext,
    ) -> Result<TargetHasher, ActionError> {
        let vcs = &self.workspace.vcs;
        let task = &self.task;
        let project = &self.project;
        let workspace = &self.workspace;
        let globset = task.create_globset()?;
        let mut hasher = TargetHasher::new();

        hasher.hash_project_deps(self.project.get_dependency_ids());
        hasher.hash_task(task);
        hasher.hash_args(&context.passthrough_args);

        // For input files, hash them with the vcs layer first
        if !task.input_paths.is_empty() {
            let mut files = convert_paths_to_strings(&task.input_paths, &workspace.root)?;

            // Sort for deterministic caching within the vcs layer
            files.sort();

            if !files.is_empty() {
                hasher.hash_inputs(vcs.get_file_hashes(&files).await?);
            }
        }

        // For input globs, it's much more performant to:
        //  `git ls-tree` -> match against glob patterns
        // Then it is to:
        //  glob + walk the file system -> `git hash-object`
        if !task.input_globs.is_empty() {
            let mut hashed_file_tree = vcs.get_file_tree_hashes(&project.source).await?;

            // Input globs are absolute paths, so we must do the same
            hashed_file_tree
                .retain(|k, _| globset.matches(&workspace.root.join(k)).unwrap_or(false));

            hasher.hash_inputs(hashed_file_tree);
        }

        // Include local file changes so that development builds work.
        // Also run this LAST as it should take highest precedence!
        let local_files = vcs.get_touched_files().await?;

        if !local_files.all.is_empty() {
            // Only hash files that are within the task's inputs
            let mut files = local_files
                .all
                .into_iter()
                .filter(|f| {
                    // Deleted files will crash `git hash-object`
                    !local_files.deleted.contains(f)
                        && globset.matches(&workspace.root.join(f)).unwrap_or(false)
                })
                .collect::<Vec<String>>();

            // Sort for deterministic caching within the vcs layer
            files.sort();

            if !files.is_empty() {
                hasher.hash_inputs(vcs.get_file_hashes(&files).await?);
            }
        }

        Ok(hasher)
    }

    pub async fn create_env_vars(&self) -> Result<HashMap<String, String>, ActionError> {
        let mut env_vars = HashMap::new();

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
        env_vars.insert("MOON_TARGET".to_owned(), self.task.target.clone());
        env_vars.insert(
            "MOON_TOOLCHAIN_DIR".to_owned(),
            path::to_string(&self.workspace.toolchain.dir)?,
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
            .create_runfile(&self.project.id, self.project)
            .await?;

        env_vars.insert(
            "MOON_PROJECT_RUNFILE".to_owned(),
            path::to_string(&runfile.path)?,
        );

        Ok(env_vars)
    }

    /// Hash the target based on all current parameters and return early
    /// if this target hash has already been cached. Based on the state
    /// of the target and project, determine the hydration strategy as well.
    pub async fn is_cached(
        &mut self,
        common_hasher: impl Hasher + Serialize,
        platform_hasher: impl Hasher + Serialize,
    ) -> Result<Option<HydrateFrom>, ActionError> {
        let hash = to_hash(&common_hasher, &platform_hasher);

        debug!(
            target: LOG_TARGET,
            "Generated hash {} for target {}",
            color::symbol(&hash),
            color::id(&self.task.target)
        );

        // Hash is the same as the previous build, so simply abort!
        // However, ensure the outputs also exist, otherwise we should hydrate.
        if self.cache.item.hash == hash && self.has_outputs() {
            debug!(
                target: LOG_TARGET,
                "Cache hit for hash {}, reusing previous build",
                color::symbol(&hash),
            );

            return Ok(Some(HydrateFrom::PreviousOutput));
        }

        self.cache.item.hash = hash.clone();

        // Refresh the hash manifest
        self.workspace
            .cache
            .create_hash_manifest(&hash, &(common_hasher, platform_hasher))
            .await?;

        // Hash exists in the cache, so hydrate from it
        if self.workspace.cache.is_hash_cached(&hash) {
            debug!(
                target: LOG_TARGET,
                "Cache hit for hash {}, hydrating from local cache",
                color::symbol(&hash),
            );

            return Ok(Some(HydrateFrom::LocalCache));
        }

        debug!(
            target: LOG_TARGET,
            "Cache miss for hash {}, continuing run",
            color::symbol(&hash),
        );

        Ok(None)
    }

    /// Return true if this target is a no-op.
    pub fn is_no_op(&self) -> bool {
        self.task.is_no_op()
    }

    /// Verify that all task outputs exist for the current target.
    /// TODO: We dont verify contents, should we?
    pub fn has_outputs(&self) -> bool {
        self.task.output_paths.iter().all(|p| p.exists())
    }

    /// Run the command as a child process and capture its output. If the process fails
    /// and `retry_count` is greater than 0, attempt the process again in case it passes.
    pub async fn run_command(
        &mut self,
        context: &ActionContext,
        command: &mut Command,
    ) -> Result<Vec<Attempt>, ActionError> {
        command.envs(self.create_env_vars().await?);

        if !context.passthrough_args.is_empty() {
            command.args(&context.passthrough_args);
        }

        if self
            .workspace
            .config
            .action_runner
            .inherit_colors_for_piped_tasks
        {
            command.inherit_colors();
        }

        let attempt_total = self.task.options.retry_count + 1;
        let mut attempt_index = 1;
        let mut attempts = vec![];
        let is_primary = context.primary_targets.contains(&self.task.target);
        let is_real_ci = is_ci() && !is_test_env();
        let output;

        // When the primary target, always stream the output for a better developer experience.
        // However, transitive targets can opt into streaming as well.
        let should_stream_output = if let Some(output_style) = &self.task.options.output_style {
            matches!(output_style, TaskOutputStyle::Stream)
        } else {
            is_primary || is_real_ci
        };

        // Transitive targets may run concurrently, so differentiate them with a prefix.
        let stream_prefix =
            if is_real_ci || !is_primary || is_primary && context.primary_targets.len() > 1 {
                Some(&self.task.target)
            } else {
                None
            };

        loop {
            let mut attempt = Attempt::new(attempt_index);

            self.print_target_label(
                // Mark primary streamed output as passed, since it may stay open forever,
                // or it may use ANSI escape codes to alter the terminal!
                if is_primary && should_stream_output {
                    Checkpoint::Pass
                } else {
                    Checkpoint::Start
                },
                &attempt,
                attempt_total,
            );

            self.print_target_command(&context.passthrough_args);

            let possible_output = if should_stream_output {
                command.exec_stream_and_capture_output(stream_prefix).await
            } else {
                command.exec_capture_output().await
            };

            attempt.done();

            match possible_output {
                // zero and non-zero exit codes
                Ok(out) => {
                    if should_stream_output {
                        self.handle_streamed_output(&attempt, attempt_total, &out);
                    } else {
                        self.handle_captured_output(&attempt, attempt_total, &out);
                    }

                    attempts.push(attempt);

                    if out.status.success() {
                        output = out;
                        break;
                    } else if attempt_index >= attempt_total {
                        return Err(ActionError::Moon(command.output_to_error(&out, false)));
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
                    return Err(ActionError::Moon(error));
                }
            }
        }

        // Write the cache with the result and output
        self.cache.item.exit_code = output.status.code().unwrap_or(0);
        self.cache.item.last_run_time = self.cache.now_millis();
        self.cache.item.stderr = output_to_string(&output.stderr);
        self.cache.item.stdout = output_to_string(&output.stdout);
        self.cache.save().await?;

        Ok(attempts)
    }

    pub fn print_cache_item(&self) {
        let item = &self.cache.item;

        self.print_output_with_style(&item.stdout, &item.stderr, item.exit_code != 0);
    }

    pub fn print_checkpoint(&self, checkpoint: Checkpoint, comment: &str) {
        println!(
            "{} {}",
            label_checkpoint(&self.task.target, checkpoint),
            color::muted(comment)
        );
    }

    pub fn print_output_with_style(&self, stdout: &str, stderr: &str, failed: bool) {
        let print_stdout = || {
            if !stdout.is_empty() {
                println!("{}", stdout);
            }
        };

        let print_stderr = || {
            if !stderr.is_empty() {
                eprintln!("{}", stderr);
            }
        };

        match self.task.options.output_style {
            // Only show output on failure
            Some(TaskOutputStyle::BufferOnlyFailure) => {
                if failed {
                    print_stdout();
                    print_stderr();
                }
            }
            // Only show the hash
            Some(TaskOutputStyle::Hash) => {
                let hash = &self.cache.item.hash;

                if !hash.is_empty() {
                    // Print to stderr so it can be captured
                    eprintln!("{}", hash);
                }
            }
            // Show nothing
            Some(TaskOutputStyle::None) => {}
            // Show output on both success and failure
            _ => {
                print_stdout();
                print_stderr();
            }
        }
    }

    pub fn print_target_command(&self, passthrough_args: &[String]) {
        if !self.workspace.config.action_runner.log_running_command {
            return;
        }

        let project = &self.project;
        let task = &self.task;

        let mut args = vec![];
        args.extend(&task.args);
        args.extend(passthrough_args);

        let command_line = if args.is_empty() {
            task.command.clone()
        } else {
            format!("{} {}", task.command, process::join_args(args))
        };

        let working_dir =
            if task.options.run_from_workspace_root || project.root == self.workspace.root {
                String::from("workspace")
            } else {
                format!(
                    ".{}{}",
                    std::path::MAIN_SEPARATOR,
                    project
                        .root
                        .strip_prefix(&self.workspace.root)
                        .unwrap()
                        .to_string_lossy(),
                )
            };

        let suffix = format!("(in {})", working_dir);
        let message = format!("{} {}", command_line, color::muted(suffix));

        println!("{}", color::muted_light(message));
    }

    pub fn print_target_label(&self, checkpoint: Checkpoint, attempt: &Attempt, attempt_total: u8) {
        let mut label = label_checkpoint(&self.task.target, checkpoint);
        let mut comments = vec![];

        if attempt.index > 1 {
            comments.push(format!("{}/{}", attempt.index, attempt_total));
        }

        if let Some(duration) = attempt.duration {
            comments.push(time::elapsed(duration));
        }

        if !comments.is_empty() {
            let metadata = color::muted(format!("({})", comments.join(", ")));

            label = format!("{} {}", label, metadata);
        };

        println!("{}", label);
    }

    // Print label *after* output has been captured, so parallel tasks
    // aren't intertwined and the labels align with the output.
    fn handle_captured_output(&self, attempt: &Attempt, attempt_total: u8, output: &Output) {
        self.print_target_label(
            if output.status.success() {
                Checkpoint::Pass
            } else {
                Checkpoint::Fail
            },
            attempt,
            attempt_total,
        );

        let stdout = output_to_string(&output.stdout);
        let stderr = output_to_string(&output.stderr);

        self.print_output_with_style(&stdout, &stderr, !output.status.success());
    }

    // Only print the label when the process has failed,
    // as the actual output has already been streamed to the console.
    fn handle_streamed_output(&self, attempt: &Attempt, attempt_total: u8, output: &Output) {
        // Transitive target finished streaming, so display the success checkpoint
        if let Some(TaskOutputStyle::Stream) = self.task.options.output_style {
            self.print_target_label(
                if output.status.success() {
                    Checkpoint::Pass
                } else {
                    Checkpoint::Fail
                },
                attempt,
                attempt_total,
            );

            // Otherwise the primary target failed for some reason
        } else if !output.status.success() {
            self.print_target_label(Checkpoint::Fail, attempt, attempt_total);
        }
    }
}
