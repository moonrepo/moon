#[cfg(windows)]
pub const BIN_NAME: &str = "moon.exe";

#[cfg(not(windows))]
pub const BIN_NAME: &str = "moon";

pub const CONFIG_DIRNAME: &str = ".moon";

pub const PROTO_CLI_VERSION: &str = "0.47.11";
