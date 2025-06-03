use crate::{is_false, is_zero};
use derive_setters::*;
use warpgate_api::{ExecCommandInput, VirtualPath, api_enum, api_struct};

api_struct!(
    #[derive(Setters)]
    #[serde(default)]
    pub struct ExecCommand {
        /// When enabled, failed command executions will
        /// not abort the moon process, and allow it to
        /// continue running.
        #[serde(skip_serializing_if = "is_false")]
        #[setters(bool)]
        pub allow_failure: bool,

        /// Cache the command based on its inputs/params and
        /// avoid re-executing until they change. Enabling
        /// this cache requires a label for debug purposes.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[setters(into, strip_option)]
        pub cache: Option<String>,

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
