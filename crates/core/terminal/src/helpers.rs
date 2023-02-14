use lazy_static::lazy_static;
use moon_logger::color;
use regex::{Captures, Regex};
use std::path::Path;

lazy_static! {
    pub static ref STYLE_TOKEN: Regex = Regex::new(r#"<(\w+)>([^<>]+)</(\w+)>"#).unwrap();
}

// https://github.com/clap-rs/clap/blob/master/src/util/mod.rs#L25
#[inline]
pub fn safe_exit(code: i32) -> ! {
    use std::io::Write;

    let _ = std::io::stdout().lock().flush();
    let _ = std::io::stderr().lock().flush();

    std::process::exit(code)
}

#[inline]
pub fn replace_style_tokens<T: AsRef<str>>(value: T) -> String {
    String::from(STYLE_TOKEN.replace_all(value.as_ref(), |caps: &Captures| {
        let token = caps.get(1).map_or("", |m| m.as_str());
        let inner = caps.get(2).map_or("", |m| m.as_str());

        match token {
            "accent" => color::muted(inner),
            "file" => color::file(inner),
            "id" => color::id(inner),
            "muted" => color::muted_light(inner),
            "path" => color::path(Path::new(inner)),
            "shell" => color::shell(inner),
            "symbol" => color::symbol(inner),
            "target" => color::target(inner),
            "url" => color::url(inner),
            _ => String::from(inner),
        }
    }))
}

#[cfg(test)]
mod test {
    use super::*;

    mod replace_style_tokens {
        use super::*;

        #[test]
        fn renders_ansi() {
            std::env::set_var("CLICOLOR_FORCE", "1");

            let list = vec!["file_path", "muted", "id", "path", "shell", "symbol"];

            for token in list {
                let value = format!("Before <{token}>inner</{token}> after");

                assert_ne!(replace_style_tokens(&value), value);
            }

            assert_eq!(
                replace_style_tokens("Before <unknown>inner</unknown> after"),
                "Before inner after"
            );
        }

        #[test]
        fn renders_multiple_ansi() {
            std::env::set_var("CLICOLOR_FORCE", "1");

            assert_ne!(
                replace_style_tokens("<muted>Before</muted> <id>inner</id> <symbol>after</symbol>"),
                "Before inner after"
            );
        }
    }
}
