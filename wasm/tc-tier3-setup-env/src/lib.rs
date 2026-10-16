// A tier 3 toolchain that also sets up an environment, which no other test
// toolchain does (tier 3 sets up the toolchain, while tier 2 setup env doesn't)
pub use tc_tier2_setup_env::*;
pub use tc_tier3::*;
