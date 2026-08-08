use crate::{is_false, is_zero};
use derive_setters::*;
use warpgate_api::{ExecCommandInput, VirtualPath, api_enum, api_struct, api_unit_enum};

api_unit_enum!(
    /// Types of caching strategies.
    pub enum CacheStrategy {
        /// Cache in-memory for the current process.
        #[default]
        Memory,

        /// Cache on disk using a hashed fingerprint,
        /// which is useful for caching across processes.
        Hash,
    }
);

api_struct!(
    /// A command to be executed on the host machine, with optional caching and retrying.
    #[derive(Setters)]
    #[serde(default)]
    pub struct ExecCommand {
        /// When enabled, failed command executions will
        /// not abort the moon process, and allow it to
        /// continue running.
        #[serde(skip_serializing_if = "is_false")]
        #[setters(bool)]
        pub allow_failure: bool,

        /// Cache the command, either in-memory or on disk.
        /// If not provided, the command will not be cached.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[setters(strip_option)]
        pub cache: Option<CacheStrategy>,

        /// The command parameters.
        #[setters(skip)]
        pub command: ExecCommandInput,

        /// List of additional inputs to gather when generating
        /// the cache key/hash.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub inputs: Vec<CacheInput>,

        /// Checkpoint label to print to the console. If not
        /// provided, will default to the command + arguments.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[setters(into, strip_option)]
        pub label: Option<String>,

        /// Execute the command in parallel.
        #[serde(skip_serializing_if = "is_false")]
        #[setters(bool)]
        pub parallel: bool,

        /// A count of how many times to retry the command
        /// if it fails to execute.
        #[serde(skip_serializing_if = "is_zero")]
        pub retry_count: u8,
    }
);

impl ExecCommand {
    /// Create a new command with the provided input.
    pub fn new(command: ExecCommandInput) -> Self {
        Self {
            allow_failure: false,
            cache: None,
            command,
            inputs: vec![],
            label: None,
            parallel: false,
            retry_count: 0,
        }
    }

    /// Return the label, or the command + arguments.
    pub fn get_label(&self) -> String {
        self.label.clone().unwrap_or_else(|| {
            format!("{} {}", self.command.command, self.command.args.join(" "))
                .trim()
                .into()
        })
    }
}

impl From<ExecCommandInput> for ExecCommand {
    fn from(input: ExecCommandInput) -> Self {
        Self::new(input)
    }
}

api_enum!(
    /// Types of inputs that can be cached.
    #[serde(tag = "type", content = "value", rename_all = "kebab-case")]
    pub enum CacheInput {
        /// Environment variable.
        EnvVar(String),

        /// SHA256 file hash.
        FileHash(VirtualPath),

        /// File size in bytes.
        FileSize(VirtualPath),

        /// File modified or created at timestamp.
        FileTimestamp(VirtualPath),
    }
);

impl CacheInput {
    /// Return the virtual path of the cache input, if applicable.
    pub fn get_virtual_path(&self) -> Option<&VirtualPath> {
        match self {
            CacheInput::FileHash(path)
            | CacheInput::FileSize(path)
            | CacheInput::FileTimestamp(path) => Some(path),
            _ => None,
        }
    }

    /// Set the virtual path of the cache input, if applicable.
    pub fn set_virtual_path(&mut self, new_path: VirtualPath) {
        match self {
            CacheInput::FileHash(path)
            | CacheInput::FileSize(path)
            | CacheInput::FileTimestamp(path) => *path = new_path,
            _ => {}
        }
    }
}
