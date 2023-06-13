use moon_common::cacheable;
use rustc_hash::FxHashMap;
use schematic::{derive_enum, Config, SchemaType, Schematic, Segment, ValidateError};

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

impl OwnersPaths {
    pub fn is_empty(&self) -> bool {
        match self {
            OwnersPaths::List(list) => list.is_empty(),
            OwnersPaths::Map(map) => map.is_empty(),
        }
    }
}

impl Default for OwnersPaths {
    fn default() -> Self {
        OwnersPaths::List(Vec::new())
    }
}

impl Schematic for OwnersPaths {
    fn generate_schema() -> SchemaType {
        SchemaType::union(vec![
            SchemaType::array(SchemaType::string()),
            SchemaType::object(
                SchemaType::string(),
                SchemaType::array(SchemaType::string()),
            ),
        ])
    }
}

fn validate_paths<C>(
    value: &OwnersPaths,
    data: &PartialOwnersConfig,
    _context: &C,
) -> Result<(), ValidateError> {
    match value {
        OwnersPaths::List(list) => {
            if !list.is_empty() && data.default_owner.is_none() {
                return Err(ValidateError::new(
                    "a default owner is required when defining a list of paths",
                ));
            }
        }
        OwnersPaths::Map(map) => {
            for (key, value) in map {
                if value.is_empty() && data.default_owner.is_none() {
                    return Err(ValidateError::with_segment(
                        "a default owner is required when defining an empty list of owners",
                        Segment::Key(key.to_owned()),
                    ));
                }
            }
        }
    };

    Ok(())
}

fn validate_required_approvals<C>(
    value: &u8,
    _data: &PartialOwnersConfig,
    _context: &C,
) -> Result<(), ValidateError> {
    if *value == 0 {
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

        #[setting(validate = validate_paths)]
        pub paths: OwnersPaths,

        // GitLab
        #[setting(default = 1, validate = validate_required_approvals)]
        pub required_approvals: u8,
    }
);
