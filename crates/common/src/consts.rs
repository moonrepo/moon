#[cfg(windows)]
pub const BIN_NAME: &str = "moon.exe";

#[cfg(not(windows))]
pub const BIN_NAME: &str = "moon";
