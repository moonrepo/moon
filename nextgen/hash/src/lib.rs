mod deps_hash;
mod hash_engine;
mod hasher;

pub use deps_hash::*;
pub use hash_engine::*;
pub use hasher::*;

#[macro_export]
macro_rules! hash_content {
    ($impl:item) => {
        #[derive(serde::Serialize)]
        #[serde(default, rename_all = "camelCase")]
        $impl
    };
}
