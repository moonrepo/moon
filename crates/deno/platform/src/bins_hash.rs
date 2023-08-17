use moon_config::BinEntry;
use moon_hash::hash_content;

hash_content!(
    pub struct DenoBinsHash<'cfg> {
        pub bins: &'cfg Vec<BinEntry>,
    }
);
