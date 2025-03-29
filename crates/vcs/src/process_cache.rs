use moon_common::is_test_env;
use moon_process::{Command, output_to_string};
use rustc_hash::FxHashMap;
use scc::HashCache;
use scc::hash_cache::Entry;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;

#[derive(Debug)]
pub struct ProcessCache {
    /// Output cache of all executed commands.
    cache: HashCache<String, Arc<String>>,

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
            cache: HashCache::new(),
            bin: bin.to_string(),
            env: FxHashMap::default(),
            workspace_root: root.to_path_buf(),
        }
    }

    pub fn create_command<I, A>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut command = Command::new(&self.bin);
        command.args(args);
        command.envs(&self.env);
        // Run from workspace root instead of git root so that we can avoid
        // prefixing all file paths to ensure everything is relative and accurate.
        command.cwd(&self.workspace_root);
        // The VCS binary should be available on the system,
        // so avoid the shell overhead
        command.without_shell();
        command
    }

    pub fn create_command_in_dir<I, A>(&self, args: I, dir: &str) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut command = self.create_command(args);

        // Run in a directory to support submodules
        if !dir.is_empty() && dir != "." {
            command.cwd(self.workspace_root.join(dir));
        }

        command
    }

    pub fn create_command_in_cwd<I, A>(&self, args: I, dir: &Path) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut command = self.create_command(args);
        command.cwd(dir);
        command
    }

    pub async fn run<I, A>(&self, args: I, trim: bool) -> miette::Result<Arc<String>>
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
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
        A: AsRef<OsStr>,
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

        // Avoid caching while testing
        if is_test_env() {
            let output = command.exec_capture_output().await?;
            let value = format_output(output);

            return Ok(value);
        }

        let cache_key = command.get_cache_key();

        // First check if the data has already been cached
        if let Some(cache) = self.cache.read_async(&cache_key, |_, v| v.clone()).await {
            return Ok(cache);
        }

        // Otherwise acquire an entry to lock the row
        let cache = match self.cache.entry_async(cache_key).await {
            Entry::Occupied(o) => o.get().clone(),
            Entry::Vacant(v) => {
                let output = command.exec_capture_output().await?;
                let cache = format_output(output);

                v.put_entry(Arc::clone(&cache));

                cache
            }
        };

        Ok(cache)
    }
}
