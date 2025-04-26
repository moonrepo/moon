use warpgate_api::{ExecCommandInput, api_struct};

api_struct!(
    pub struct ExecCommand {
        /// Cache the command based on its input and
        /// avoid re-executing until the input changes.
        pub cache: bool,

        /// Input for the command parameters.
        pub input: ExecCommandInput,

        /// Execute command in parallel.
        pub parallel: bool,
    }
);

impl ExecCommand {
    pub fn new(input: ExecCommandInput) -> Self {
        Self {
            cache: true,
            input,
            parallel: false,
        }
    }

    pub fn cache(mut self) -> Self {
        self.cache = true;
        self
    }

    pub fn no_cache(mut self) -> Self {
        self.cache = false;
        self
    }

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
