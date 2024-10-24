mod cache_item;
mod cache_mode;

pub use cache_item::*;
pub use cache_mode::*;

#[macro_export]
macro_rules! cache_item {
    ($item:item) => {
        #[derive(Debug, Default, /* Eq, */ PartialEq, serde::Deserialize, serde::Serialize)]
        #[serde(default, rename_all = "camelCase")]
        $item
    };
}

cache_item!(
    pub struct CommonCacheState {
        pub last_hash: String,
    }
);
