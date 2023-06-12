mod hash_engine;
mod hasher;

pub use hash_engine::*;
pub use hasher::*;

#[macro_export]
macro_rules! content_hashable {
    ($impl:item) => {
        #[derive(serde::Serialize)]
        #[serde(default, rename_all = "camelCase")]
        $impl
    };
}
