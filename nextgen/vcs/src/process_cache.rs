use dashmap::DashMap;
use moon_process::{output_to_string, Command};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ProcessCache {
    /// Output cache of all executed commands.
    cache: DashMap<String, Arc<String>>,

    /// Avoids concurrent commands of the same cache key from all
    /// reading/writing the same data, by storing them in a queue.
    queue: DashMap<String, Mutex<()>>,

    /// Binary/command to run.
    pub bin: String,

    /// Root of the moon workspace, and where to run commands.
    pub root: PathBuf,
}

impl ProcessCache {
    pub fn new(bin: &str, root: &Path) -> Self {
        Self {
            cache: DashMap::new(),
            queue: DashMap::new(),
            bin: bin.to_string(),
            root: root.to_path_buf(),
        }
    }

    pub fn create_command<I, A>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut command = Command::new(&self.bin);
        command.args(args);
        // Run from workspace root instead of git root so that we can avoid
        // prefixing all file paths to ensure everything is relative and accurate.
        command.cwd(&self.root);
        // The VCS binary should be available on the system,
        // so avoid the shell overhead
        command.without_shell();
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
        command: Command,
        trim: bool,
        format: impl FnOnce(String) -> String,
    ) -> miette::Result<Arc<String>> {
        let mut executor = command.create_async();
        let cache_key = executor.inspector.get_cache_key();

        // First check if the data has already been cached
        if let Some(cache) = self.cache.get(&cache_key) {
            return Ok(Arc::clone(cache.value()));
        }

        // Otherwise wait in the queue for the cache to be written
        let entry = self
            .queue
            .entry(cache_key.clone())
            .or_insert_with(|| Mutex::new(()));
        let _guard = entry.value().lock().await;

        // Check the cache again incase of a lock race condition
        if let Some(cache) = self.cache.get(&cache_key) {
            return Ok(Arc::clone(cache.value()));
        }

        // Otherwise write to the cache!
        let output = executor.exec_capture_output().await?;
        let value = output_to_string(&output.stdout);
        let cache = Arc::new(format(if trim { value.trim().to_owned() } else { value }));

        self.cache.insert(cache_key, Arc::clone(&cache));

        Ok(cache)
    }
}
