#[cfg(not(target_arch = "wasm32"))]
mod env;
mod id;
mod macros;
pub mod path;

#[cfg(not(target_arch = "wasm32"))]
pub use env::*;
pub use id::*;
pub use starbase_styles::*;

// https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.max_blocking_threads
pub const BLOCKING_THREAD_COUNT: usize = 512;
