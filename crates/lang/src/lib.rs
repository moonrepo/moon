pub type StaticString = &'static str;

pub type StaticStringList = &'static [StaticString];

pub struct PackageManager {
    pub config_filenames: StaticStringList,

    pub default_version: StaticString,

    pub lock_filenames: StaticStringList,

    pub manifest_filename: StaticString,
}

pub struct VersionManager {
    pub config_filename: Option<StaticString>,

    pub version_filename: StaticString,
}
