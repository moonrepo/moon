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

// $ENV_VAR
pub static ENV_VAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(?:\\$(?P<namespace>E:|env::|env:|env.|ENV.)?(?P<name>[A-Z0-9_]+))").unwrap()
});

// ${ENV_VAR}
pub static ENV_VAR_BRACKETS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?:\\$\\{(?P<namespace>E:|env::|env:|env.|ENV.)?(?P<name>[A-Z0-9_]+)(?P<flag>[!?:]{1})?(?P<fallback>[^}]*)?\\})",
    )
    .unwrap()
});

pub fn contains_env_var(value: impl AsRef<str>) -> bool {
    ENV_VAR.is_match(value.as_ref()) || ENV_VAR_BRACKETS.is_match(value.as_ref())
}
