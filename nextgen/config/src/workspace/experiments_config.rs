use moon_common::cacheable;
use schematic::{env, Config};

cacheable!(
    #[derive(Config, Debug)]
    pub struct ExperimentsConfig {
        #[setting(default = true, env = "MOON_DISABLE_OVERLAPPING_OUTPUTS", parse_env = env::parse_bool)]
        pub project_graph_output_boundaries: bool,
    }
);
