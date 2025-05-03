use warpgate_api::{ExecCommandInput, VirtualPath, api_enum, api_struct};

api_struct!(
    #[serde(default)]
    pub struct ExecCommand {
        /// When enabled, failed command executions will
        /// not abort the moon process, and allow it to
        /// continue running.
        pub allow_failure: bool,

        /// Cache the command based on its input/params and
        /// avoid re-executing until they change. Enabling
        /// this cache requires a label for debug purposes.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache: Option<String>,

        /// The command parameters.
        pub command: ExecCommandInput,

        /// List of additional inputs to gather when generating
        /// the cache key/hash.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub inputs: Vec<CacheInput>,

        /// Execute the command in parallel.
        pub parallel: bool,
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
            parallel: false,
        }
    }

    /// Allow failures to not abort the moon process.
    pub fn allow_failure(mut self) -> Self {
        self.allow_failure = true;
        self
    }

    /// Enable caching.
    pub fn cache(mut self, label: impl AsRef<str>) -> Self {
        self.cache = Some(label.as_ref().into());
        self
    }

    /// Disallow failures and abort the moon process.
    pub fn disallow_failure(mut self) -> Self {
        self.allow_failure = false;
        self
    }

    /// Disable caching.
    pub fn no_cache(mut self) -> Self {
        self.cache = None;
        self
    }

    /// Set a list of inputs to cache with.
    pub fn inputs(mut self, inputs: Vec<CacheInput>) -> Self {
        self.inputs = inputs;
        self
    }

    /// Run in parallel.
    pub fn parallel(mut self) -> Self {
        self.parallel = true;
        self
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
