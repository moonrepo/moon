pub use semver::*;

#[inline]
pub fn extract_major_version<T: AsRef<str>>(version: T) -> u64 {
    match semver::Version::parse(version.as_ref()) {
        Ok(v) => v.major,
        Err(_) => 0,
    }
}

#[inline]
pub fn satisfies_range(version: &str, range: &str) -> bool {
    if let Ok(req) = VersionReq::parse(range) {
        return satisfies_requirement(version, &req);
    }

    false
}

#[inline]
pub fn satisfies_requirement(version: &str, req: &VersionReq) -> bool {
    if let Ok(ver) = Version::parse(version) {
        return req.matches(&ver);
    }

    false
}
