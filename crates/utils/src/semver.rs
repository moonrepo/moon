pub use semver::*;

pub fn extract_major_version(version: &str) -> u64 {
    match semver::Version::parse(version) {
        Ok(v) => v.major,
        Err(_) => 0,
    }
}
