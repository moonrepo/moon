use lazy_static::lazy_static;
use moon_logger::color;
use regex::{Captures, Regex};
use std::path::Path;

lazy_static! {
    pub static ref STYLE_TOKEN: Regex = Regex::new(r#"<(\w+)>(.+)</(\w+)>"#).unwrap();
}

pub fn replace_style_tokens(value: &str) -> String {
    let message = STYLE_TOKEN.replace(value, |caps: &Captures| {
        let token = caps.get(1).map_or("", |m| m.as_str());
        let inner = caps.get(2).map_or("", |m| m.as_str());

        match token {
            "file_path" => color::file_path(Path::new(inner)),
            "path" => color::path(inner),
            "shell" => color::shell(inner),
            "symbol" => color::symbol(inner),
            "url" => color::url(inner),
            _ => String::from(inner),
        }
    });

    String::from(message)
}

#[cfg(test)]
mod test {
    use super::*;

    mod replace_style_tokens {
        use super::*;

        #[test]
        fn renders_ansi() {
            let list = vec!["file_path", "path", "shell", "symbol"];

            for token in list {
                let value = format!("Before <{}>inner</{}> after", token, token);

                assert_ne!(replace_style_tokens(&value), value);
            }

            assert_eq!(
                replace_style_tokens("Before <unknown>inner</unknown> after"),
                "Before inner after"
            );
        }
    }
}
