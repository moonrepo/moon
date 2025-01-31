use moon_process::{output_to_string, Command};
use scc::hash_cache::Entry;
use scc::HashCache;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct ProcessCache {
    /// Output cache of all executed commands.
    cache: HashCache<String, Arc<String>>,

    /// Binary/command to run.
    pub bin: String,

    /// Root of the moon workspace, and where to run commands.
    pub root: PathBuf,
}

impl ProcessCache {
    pub fn new(bin: &str, root: &Path) -> Self {
        Self {
            cache: HashCache::new(),
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

    pub fn create_command_in_dir<I, A>(&self, args: I, dir: &str) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut command = self.create_command(args);

        // Run in a directory to support submodules
        if !dir.is_empty() && dir != "." {
            command.cwd(self.root.join(dir));
        }

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
                let value = output_to_string(&output.stdout);
                let cache = Arc::new(format(if trim { value.trim().to_owned() } else { value }));

                v.put_entry(Arc::clone(&cache));

                cache
            }
        };

        Ok(cache)
    }

    pub async fn run_command_without_cache(
        &self,
        mut command: Command,
        trim: bool,
    ) -> miette::Result<Arc<String>> {
        let output = command.exec_capture_output().await?;
        let value = output_to_string(&output.stdout);

        Ok(Arc::new(if trim { value.trim().to_owned() } else { value }))
    }
}
