use moon_lang::{DependencyManager, Language, VersionManager};

pub const PYTHON: Language = Language {
    binary: "python",
    file_exts: &["py", "pyc", "pyo", "pyd"],
    vendor_bins_dir: None,
    vendor_dir: None,
};

// Package managers

pub const PIP: DependencyManager = DependencyManager {
    binary: "pip",
    config_files: &["constraints.txt"],
    lockfile: ".pylock.toml", // https://peps.python.org/pep-0665/
    manifest: "requirements.txt",
};

pub const PIPENV: DependencyManager = DependencyManager {
    binary: "pipenv",
    config_files: &[],
    lockfile: "Pipfile.lock",
    manifest: "Pipfile",
};

// Version managers

pub const PYENV: VersionManager = VersionManager {
    binary: "pyenv",
    version_file: ".python-version",
};
