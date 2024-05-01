use crate::errors::RunnerError;
use crate::run_state::{load_output_logs, save_output_logs, RunTargetState};
use moon_action::{ActionNode, ActionStatus, Attempt};
use moon_action_context::{ActionContext, TargetState};
use moon_cache_item::CacheItem;
use moon_config::{TaskOptionAffectedFiles, TaskOutputStyle};
use moon_console::{Checkpoint, Console};
use moon_emitter::{Emitter, Event, EventFlow};
use moon_hash::ContentHasher;
use moon_logger::{debug, warn};
use moon_platform::PlatformManager;
use moon_platform_runtime::Runtime;
use moon_process::{args, output_to_error, output_to_string, Command, Output, Shell};
use moon_project::Project;
use moon_target::{TargetError, TargetScope};
use moon_task::Task;
use moon_task_hasher::TaskHasher;
use moon_tool::get_proto_env_vars;
use moon_utils::{is_ci, is_test_env, path, time};
use moon_workspace::Workspace;
use rustc_hash::FxHashMap;
use starbase_styles::color;
use starbase_utils::glob;
use std::collections::BTreeMap;
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

    console: Arc<Console>,

    task: &'a Task,

    workspace: &'a Workspace,
}

impl<'a> Runner<'a> {
    pub fn new(
        emitter: &'a Emitter,
        workspace: &'a Workspace,
        project: &'a Project,
        task: &'a Task,
        console: Arc<Console>,
    ) -> miette::Result<Runner<'a>> {
        let mut cache = workspace
            .cache_engine
            .state
            .load_target_state::<RunTargetState>(&task.target)?;

        if cache.data.target.is_empty() {
            cache.data.target = task.target.to_string();
        }

        Ok(Runner {
            cache,
            node: Arc::new(ActionNode::None),
            emitter,
            project,
            console,
            task,
            workspace,
        })
    }

    pub async fn archive_outputs(&self) -> miette::Result<()> {
        Ok(())
    }

    pub async fn hydrate(&self, from: HydrateFrom) -> miette::Result<ActionStatus> {
        let mut comments = vec![match from {
            HydrateFrom::LocalCache => "cached",
            HydrateFrom::RemoteCache => "cached from remote",
            HydrateFrom::PreviousOutput => "cached from previous run",
        }
        .to_owned()];

        if self.should_print_short_hash() {
            comments.push(self.get_short_hash().to_owned());
        }

        self.print_checkpoint(Checkpoint::RunPassed, comments)?;
        self.print_cache_item()?;

        Ok(if matches!(from, HydrateFrom::RemoteCache) {
            ActionStatus::CachedFromRemote
        } else {
            ActionStatus::Cached
        })
    }

    pub async fn hydrate_outputs(&self) -> miette::Result<()> {
        Ok(())
    }

    pub async fn hash_common_target(
        &self,
        _context: &ActionContext,
        _hasher: &mut ContentHasher,
    ) -> miette::Result<()> {
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

        let command = PlatformManager::read()
            .get(task.platform)?
            .create_run_target_command(context, project, task, runtime, working_dir)
            .await?;

        Ok(command)
    }

    pub async fn create_env_vars(&self, _command: &mut Command) -> miette::Result<()> {
        Ok(())
    }

    pub fn get_short_hash(&self) -> &str {
        if self.cache.data.hash.is_empty() {
            "" // Empty when cache is disabled
        } else {
            &self.cache.data.hash[0..8]
        }
    }

    pub fn has_outputs(&self, _bypass_globs: bool) -> miette::Result<bool> {
        Ok(true)
    }

    pub fn is_archivable(&self) -> miette::Result<bool> {
        Ok(true)
    }

    pub async fn is_cached(
        &mut self,
        _context: &ActionContext,
        _runtime: &Runtime,
    ) -> miette::Result<Option<HydrateFrom>> {
        Ok(None)
    }

    pub async fn run_command(
        &mut self,
        _context: &ActionContext,
        _command: &mut Command,
    ) -> miette::Result<Vec<Attempt>> {
        Ok(vec![])
    }

    pub async fn create_and_run_command(
        &mut self,
        context: &ActionContext,
        runtime: &Runtime,
    ) -> miette::Result<Vec<Attempt>> {
        Ok(vec![])
    }

    pub fn print_cache_item(&self) -> miette::Result<()> {
        let item = &self.cache;
        let (stdout, stderr) = load_output_logs(item.get_dir())?;

        self.print_output_with_style(&stdout, &stderr, item.data.exit_code != 0)?;

        Ok(())
    }

    pub fn print_checkpoint<C: AsRef<[String]>>(
        &self,
        checkpoint: Checkpoint,
        comments: C,
    ) -> miette::Result<()> {
        self.console
            .out
            .print_checkpoint_with_comments(checkpoint, &self.task.target, comments)?;

        Ok(())
    }

    pub fn print_output_with_style(
        &self,
        stdout: &str,
        stderr: &str,
        failed: bool,
    ) -> miette::Result<()> {
        let print_stdout = || -> miette::Result<()> { self.console.out.write_line(stdout) };
        let print_stderr = || -> miette::Result<()> { self.console.err.write_line(stderr) };

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
                    self.console.err.write_line(hash)?;
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

        self.console.out.write_line(message)?;

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

        self.print_checkpoint(checkpoint, comments)?;

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
