use moon_common::{is_daemon_env, is_test_env};
use moon_process::{Command, CommandArg, Output, output_to_string};
use rustc_hash::FxHashMap;
use scc::hash_cache::Entry;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct ProcessCache {
    /// Output cache of all executed commands.
    cache: scc::HashCache<String, Arc<String>>,

    /// Binary/command to run.
    pub bin: String,

    /// Environment variables to inject into each command.
    pub env: FxHashMap<String, String>,

    /// Root of the moon workspace, and where to run commands.
    pub workspace_root: PathBuf,
}

impl ProcessCache {
    pub fn new(bin: &str, root: &Path) -> Self {
        Self {
            cache: scc::HashCache::new(),
            bin: bin.to_string(),
            env: FxHashMap::default(),
            workspace_root: root.to_path_buf(),
        }
    }

    pub fn create_command<I, A>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = A>,
        A: Into<CommandArg>,
    {
        let mut command = Command::new(&self.bin);
        command.args(args);
        command.envs(&self.env);
        // Strip git's per-invocation environment (no-op outside of hooks).
        // When moon runs inside a git hook, git exports GIT_DIR / GIT_INDEX_FILE /
        // etc. into the environment, often as paths RELATIVE to the hook's repo
        // root. Because we run commands with the cwd set to a project directory
        // (see `create_command_in_cwd`), an inherited relative
        // `GIT_INDEX_FILE=.git/index` resolves against the wrong directory — and
        // for projects that are git submodules, `.git` is a FILE (a gitdir
        // pointer), so the call fails with
        // `fatal: .git/index: index file open failed: Not a directory`,
        // aborting the hook. Removing these lets each git call discover its own
        // repository from its cwd, which is what moon intends.
        command.envs_remove([
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_PREFIX",
            "GIT_OBJECT_DIRECTORY",
            "GIT_COMMON_DIR",
        ]);
        // Run from workspace root instead of git root so that we can avoid
        // prefixing all file paths to ensure everything is relative and accurate.
        command.cwd(&self.workspace_root);
        // The VCS binary should be available on the system,
        // so avoid the shell overhead
        command.no_shell();
        command
    }

    pub fn create_command_in_cwd<I, A>(&self, args: I, dir: &Path) -> Command
    where
        I: IntoIterator<Item = A>,
        A: Into<CommandArg>,
    {
        let mut command = self.create_command(args);
        command.cwd(dir);
        command
    }

    pub async fn run<I, A>(&self, args: I, trim: bool) -> miette::Result<Arc<String>>
    where
        I: IntoIterator<Item = A>,
        A: Into<CommandArg>,
    {
        self.run_command(self.create_command(args), trim).await
    }

    pub async fn run_with_formatter<I, A>(
        &self,
        args: I,
        trim: bool,
        format: impl FnOnce(String) -> String,
    ) -> miette::Result<Arc<String>>
    where
        I: IntoIterator<Item = A>,
        A: Into<CommandArg>,
    {
        self.run_command_with_formatter(self.create_command(args), trim, format)
            .await
    }

    pub async fn run_command(&self, command: Command, trim: bool) -> miette::Result<Arc<String>> {
        self.run_command_with_formatter(command, trim, |s| s).await
    }

    pub async fn run_command_with_formatter(
        &self,
        mut command: Command,
        trim: bool,
        format: impl FnOnce(String) -> String,
    ) -> miette::Result<Arc<String>> {
        let format_output = |output: Output| {
            let value = output_to_string(&output.stdout);
            Arc::new(format(if trim { value.trim().to_owned() } else { value }))
        };

        // Avoid caching while testing or within the daemon
        if is_test_env() || is_daemon_env() {
            let output = command.exec_capture_output().await?;
            let value = format_output(output);

            return Ok(value);
        }

        let cache = match self.cache.entry_async(command.get_cache_key()).await {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let output = command.exec_capture_output().await?;
                let cache = format_output(output);

                entry.put_entry(Arc::clone(&cache));

                cache
            }
        };

        Ok(cache)
    }
}
