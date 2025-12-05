mod hasher;

pub use hasher::*;

#[macro_export]
macro_rules! hash_fingerprint {
    ($impl:item) => {
        #[derive(serde::Serialize)]
        #[serde(default, rename_all = "camelCase")]
        $impl
    };
}
