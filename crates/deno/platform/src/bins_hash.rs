use moon_config::BinEntry;
use moon_hash::content_hashable;

content_hashable!(
    pub struct DenoBinsHash<'cfg> {
        pub bins: &'cfg Vec<BinEntry>,
    }
);
