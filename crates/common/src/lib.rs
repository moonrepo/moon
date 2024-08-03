pub mod consts;
#[cfg(not(target_arch = "wasm32"))]
mod env;
mod id;
mod macros;
pub mod path;
pub mod serde;

#[cfg(not(target_arch = "wasm32"))]
pub use env::*;
pub use id::*;
pub use starbase_styles::*;

pub fn supports_pkl_configs() -> bool {
    std::env::var("MOON_EXPERIMENT_PKL_CONFIG").is_ok_and(|value| value == "1" || value == "true")
}
