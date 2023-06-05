use moon_common::cacheable;
use rustc_hash::FxHashMap;
use schematic::{derive_enum, Config, ValidateError};

derive_enum!(
    #[serde(
        untagged,
        expecting = "expected a list of paths, or a map of paths to owners"
    )]
    pub enum OwnersPaths {
        List(Vec<String>),
        Map(FxHashMap<String, Vec<String>>),
    }
);

impl Default for OwnersPaths {
    fn default() -> Self {
        OwnersPaths::List(Vec::new())
    }
}

fn validate_required_approvals<D, C>(
    value: &u8,
    _data: &D,
    _context: &C,
) -> Result<(), ValidateError> {
    if *value <= 0 {
        return Err(ValidateError::new("at least 1 approver is required"));
    }

    Ok(())
}

cacheable!(
    #[derive(Clone, Config, Debug)]
    pub struct OwnersConfig {
        // Bitbucket
        pub custom_groups: FxHashMap<String, Vec<String>>,

        pub default_owner: Option<String>,

        // GitLab
        pub optional: bool,

        pub paths: OwnersPaths,

        // GitLab
        #[setting(default = 1, validate = validate_required_approvals)]
        pub required_approvals: u8,
    }
);
