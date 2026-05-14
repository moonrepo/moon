mod content_hash;
mod content_hasher;
mod hash_error;

pub use content_hash::*;
pub use content_hasher::*;
pub use hash_error::*;

#[macro_export]
macro_rules! hash_fingerprint {
    ($impl:item) => {
        #[derive(serde::Serialize)]
        #[serde(default, rename_all = "camelCase")]
        $impl
    };
}
