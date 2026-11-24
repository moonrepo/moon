mod dotenv;
mod dotenv_error;
mod env_scanner;
mod env_substitutor;
mod global_bag;

pub use dotenv::*;
pub use dotenv_error::*;
pub use env_scanner::*;
pub use env_substitutor::*;
pub use global_bag::*;

use regex::Regex;
use std::sync::LazyLock;

// $E: = Elvish
// $env: = PowerShell
// $env:: = Ion
// $env. = Nu
// $ENV. = Murex

// A name must contain a letter or number, as `$_` is set by Bash to the program
// it runs, and `$__proto__` would substitute `$__`. Keep in sync with `moon_config`!
// $ENV_VAR
pub static ENV_VAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(?:\\$(?P<namespace>E:|env::|env:|env\\.|ENV\\.)?(?P<name>_*[A-Z0-9][A-Z0-9_]*))")
        .unwrap()
});

// ${ENV_VAR}
pub static ENV_VAR_BRACKETS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?:\\$\\{(?P<namespace>E:|env::|env:|env\\.|ENV\\.)?(?P<name>_*[A-Z0-9][A-Z0-9_]*)(?P<flag>!|\\?|:-|:\\+|-|\\+|:)?(?P<fallback>[^}]*)?\\})",
    )
    .unwrap()
});

pub fn contains_env_var(value: impl AsRef<str>) -> bool {
    ENV_VAR.is_match(value.as_ref()) || ENV_VAR_BRACKETS.is_match(value.as_ref())
}

// Env inheritance in order of priority:
//
//  1) Global/shell vars
//      - So that `KEY=value moon ...` works
//      - Cannot access task/dotenv vars
//  2) Task vars
//      - Can substitute with globals/self
//      - Cannot access dotenv vars
//  3) Dotenv vars
//      - Can substitute with globals/task/self
//      - Can access previous dotenv vars
