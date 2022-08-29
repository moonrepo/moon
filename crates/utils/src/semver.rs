pub use semver::*;

pub fn extract_major_version(version: &str) -> u64 {
    match semver::Version::parse(version) {
        Ok(v) => v.major,
        Err(_) => 0,
    }
}

pub fn satisfies_range(version: &str, range: &str) -> bool {
    if let Ok(req) = VersionReq::parse(range) {
        return satisfies_requirement(version, &req);
    }

    false
}

pub fn satisfies_requirement(version: &str, req: &VersionReq) -> bool {
    if let Ok(ver) = Version::parse(version) {
        // println!("{:#?}", ver);
        // println!("{:#?}", req);
        // println!("{}", req.matches(&ver));

        return req.matches(&ver);
    }

    false
}
