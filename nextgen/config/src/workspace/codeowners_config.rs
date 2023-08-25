use moon_common::cacheable;
use rustc_hash::FxHashMap;
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum CodeownersOrderBy {
        #[default]
        FileSource,
        ProjectName,
    }
);

cacheable!(
    #[derive(Config, Debug)]
    pub struct CodeownersConfig {
        pub global_paths: FxHashMap<String, Vec<String>>,

        pub order_by: CodeownersOrderBy,

        pub sync_on_run: bool,
    }
);
