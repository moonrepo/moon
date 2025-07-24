mod input;
mod output_path;
mod poly;
mod portable_path;

pub use input::*;
pub use output_path::*;
pub use poly::*;
pub use portable_path::*;

use schematic::ParseError;

#[derive(Default)]
pub struct Uri {
    pub scheme: String,
    pub path: String,
    pub query: Vec<(String, String)>,
}

impl Uri {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ParseError> {
        let mut uri = Self::default();

        let Some((scheme, suffix)) = value.as_ref().split_once("://") else {
            return Err(ParseError::new("missing scheme (protocol before ://)"));
        };

        uri.scheme = scheme.into();

        if let Some(index) = suffix.rfind('?')
            && index != suffix.len() - 1
        {
            uri.path = suffix[0..index].into();

            for segment in suffix[index + 1..].split('&') {
                match segment.split_once('=') {
                    Some((key, value)) => {
                        uri.query.push((key.into(), value.into()));
                    }
                    None => {
                        uri.query.push((segment.into(), String::new()));
                    }
                };
            }
        } else {
            uri.path = suffix.into();
        }

        Ok(uri)
    }
}

pub fn is_false(value: &bool) -> bool {
    !(*value)
}
