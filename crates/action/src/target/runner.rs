use crate::action::Attempt;
use crate::context::ActionContext;
use crate::errors::ActionError;
use moon_cache::{CacheItem, RunTargetState};
use moon_error::MoonError;
use moon_hasher::{convert_paths_to_strings, to_hash, Hasher, TargetHasher};
use moon_logger::{color, debug, warn};
use moon_project::Project;
use moon_task::Task;
use moon_terminal::{label_checkpoint, Checkpoint};
use moon_utils::{
    is_ci, is_test_env, path,
    process::{self, output_to_string, Command, Output},
    time,
};
use moon_workspace::Workspace;
use serde::Serialize;
use std::collections::HashMap;

const LOG_TARGET: &str = "moon:action:run-target";

pub struct TargetRunner<'a> {
    pub cache: CacheItem<RunTargetState>,

    project: &'a Project,

    target_id: &'a str,

    task: &'a Task,

    workspace: &'a Workspace,
}

impl<'a> TargetRunner<'a> {
    pub async fn new(
        workspace: &'a Workspace,
        project: &'a Project,
        task: &'a Task,
        target_id: &'a str,
    ) -> Result<TargetRunner<'a>, MoonError> {
        Ok(TargetRunner {
            cache: workspace.cache.cache_run_target_state(target_id).await?,
            project,
            target_id,
            task,
            workspace,
        })
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
    /// if this target hash has already been cached.
    pub async fn is_cached(
        &mut self,
        context: &ActionContext,
        platform_hasher: impl Hasher + Serialize,
    ) -> Result<bool, ActionError> {
        let base_hasher = self.create_base_hasher(context).await?;
        let hash = to_hash(&base_hasher, &platform_hasher);

        debug!(
            target: LOG_TARGET,
            "Generated hash {} for target {}",
            color::symbol(&hash),
            color::id(&self.target_id)
        );

        if self.cache.item.hash == hash {
            return Ok(true);
        }

        self.cache.item.hash = hash;

        // self.workspace
        //     .cache
        //     .save_hash(&hash, [base_hasher, platform_hasher])
        //     .await?;

        Ok(false)
    }

    /// Return true if this target is a no-op.
    pub fn is_no_op(&self) -> bool {
        self.task.is_no_op()
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
        let is_primary = context.primary_targets.contains(self.target_id);
        let is_real_ci = is_ci() && !is_test_env();
        let stream_output = is_primary || is_real_ci;
        let output;

        loop {
            let mut attempt = Attempt::new(attempt_index);

            let possible_output = if stream_output {
                // Print label *before* output is streamed since it may stay open forever,
                // or it may use ANSI escape codes to alter the terminal.
                self.print_target_label(Checkpoint::Pass, &attempt, attempt_total);
                self.print_target_command(&context.passthrough_args);

                // If this target matches the primary target (the last task to run),
                // then we want to stream the output directly to the parent (inherit mode).
                command
                    .exec_stream_and_capture_output(if is_real_ci {
                        Some(self.target_id)
                    } else {
                        None
                    })
                    .await
            } else {
                self.print_target_label(Checkpoint::Start, &attempt, attempt_total);
                self.print_target_command(&context.passthrough_args);

                // Otherwise we run the process in the background and write the output
                // once it has completed.
                command.exec_capture_output().await
            };

            attempt.done();

            match possible_output {
                // zero and non-zero exit codes
                Ok(out) => {
                    if stream_output {
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
                            color::target(&self.target_id),
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

        if !item.stderr.is_empty() {
            eprintln!("{}", item.stderr.trim());
            eprintln!();
        }

        if !item.stdout.is_empty() {
            println!("{}", item.stdout.trim());
            println!();
        }
    }

    pub fn print_checkpoint(&self, checkpoint: Checkpoint, comment: &str) {
        println!(
            "{} {}",
            label_checkpoint(&self.target_id, checkpoint),
            color::muted(comment)
        );
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
        let failed = matches!(checkpoint, Checkpoint::Fail);
        let mut label = label_checkpoint(&self.target_id, checkpoint);
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

        if failed {
            eprintln!("{}", label);
        } else {
            println!("{}", label);
        }
    }

    /// Create a hasher that is shared amongst all platforms.
    /// Primarily includes task information.
    async fn create_base_hasher(
        &self,
        context: &ActionContext,
    ) -> Result<TargetHasher, ActionError> {
        let vcs = &self.workspace.vcs;
        let task = &self.task;
        let project = &self.project;
        let workspace = &self.workspace;
        let globset = task.create_globset()?;
        let mut hasher = TargetHasher::new();

        hasher.hash_project_deps(self.project.get_dependencies());
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

        let stderr = output_to_string(&output.stderr);
        let stdout = output_to_string(&output.stdout);

        if !stderr.is_empty() {
            eprintln!("{}", stderr.trim());
            eprintln!();
        }

        if !stdout.is_empty() {
            println!("{}", stdout.trim());
            println!();
        }
    }

    // Only print the label when the process has failed,
    // as the actual output has already been streamed to the console.
    fn handle_streamed_output(&self, attempt: &Attempt, attempt_total: u8, output: &Output) {
        if !output.status.success() {
            self.print_target_label(Checkpoint::Fail, attempt, attempt_total);
        }
    }
}
