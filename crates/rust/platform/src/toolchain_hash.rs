use moon_config::BinEntry;
use moon_hash::hash_content;

hash_content!(
    pub struct RustToolchainHash<'cfg> {
        pub bins: &'cfg Vec<BinEntry>,
        pub components: &'cfg Vec<String>,
        pub targets: &'cfg Vec<String>,
    }
);
