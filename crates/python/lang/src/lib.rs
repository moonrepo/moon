use moon_lang::{DependencyManager, Language, VersionManager};

pub const PYTHON: Language = Language {
    binary: "python",
    default_version: "3.11.1",
    file_exts: &["py", "pyc", "pyo", "pyd"],
    vendor_bins_dir: None,
    vendor_dir: None,
};

// Package managers

pub const PIP: DependencyManager = DependencyManager {
    binary: "pip",
    config_files: &["constraints.txt"],
    default_version: "22.3.1",
    lockfile: ".pylock.toml", // https://peps.python.org/pep-0665/
    manifest: "requirements.txt",
};

pub const PIPENV: DependencyManager = DependencyManager {
    binary: "pipenv",
    config_files: &[],
    default_version: "2022.11.30",
    lockfile: "Pipfile.lock",
    manifest: "Pipfile",
};

// Version managers

pub const PYENV: VersionManager = VersionManager {
    binary: "pyenv",
    version_file: ".python-version",
};
