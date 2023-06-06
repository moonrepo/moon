mod hash_engine;
mod hash_error;
mod hasher;

pub use hash_engine::*;
pub use hash_error::*;
pub use hasher::*;

#[macro_export]
macro_rules! content_hashable {
    ($impl:item) => {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        $impl
    };
}
