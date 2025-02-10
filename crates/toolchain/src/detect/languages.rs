pub type StaticString = &'static str;
pub type StaticStringList = &'static [StaticString];

pub static BUN: StaticStringList = &["bunfig.toml", "bun.lock", "bun.lockb", ".bunrc"];

pub static DENO: StaticStringList = &["deno.json", "deno.jsonc", "deno.lock", ".dvmrc"];

pub static GO: StaticStringList = &["go.mod", "go.sum", "g.lock", ".gvmrc", ".go-version"];

pub static NODE: StaticStringList = &[
    "package.json",
    ".nvmrc",
    ".node-version",
    // npm
    "package-lock.json",
    ".npmrc",
    // pnpm
    ".pnpmfile.cjs",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    // yarn
    "yarn.lock",
    ".yarn",
    ".yarnrc",
    ".yarnrc.yml",
];

pub static PHP: StaticStringList = &[
    "composer.json",
    "composer.lock",
    ".phpenv-version",
    ".phpbrewrc",
];

pub static PYTHON: StaticStringList = &[
    "requirements.txt",
    "constraints.txt",
    "pyproject.toml",
    ".pylock.toml",
    ".python-version",
    ".venv",
    // pip
    "Pipfile",
    "Pipfile.lock",
    // poetry
    "poetry.toml",
    "poetry.lock",
    // uv
    "uv.toml",
    "uv.lock",
];

pub static RUBY: StaticStringList = &["Gemfile", "Gemfile.lock", ".bundle", ".ruby-version"];

pub static RUST: StaticStringList = &[
    "Cargo.toml",
    "Cargo.lock",
    ".cargo",
    "rust-toolchain.toml",
    "rust-toolchain",
];

pub static TYPESCRIPT: StaticStringList = &["tsconfig.json", "tsconfig.tsbuildinfo"];
