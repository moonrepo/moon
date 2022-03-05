pub mod fs;
pub mod process;
pub mod regex;
pub mod test;
pub mod time;

use std::env;

#[macro_export]
macro_rules! string_vec {
    () => {{
        Vec::<String>::new()
    }};
    ($($item:expr),+ $(,)?) => {{
        vec![
            $( String::from($item), )*
        ]
    }};
}

pub fn is_ci() -> bool {
    env::var("CI").is_ok()
}
