use crate::dotenv_error::DotEnvError;
use crate::env_substitutor::EnvSubstitutor;
use crate::global_bag::GlobalEnvBag;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum QuoteStyle {
    Single,
    Double,
    Unquoted,
}

#[derive(Default)]
pub struct DotEnv<'a> {
    global_vars: Option<&'a GlobalEnvBag>,
    local_vars: FxHashMap<&'a String, &'a Option<String>>,
}

impl<'a> DotEnv<'a> {
    pub fn with_global_vars(mut self, vars: &'a GlobalEnvBag) -> Self {
        self.global_vars = Some(vars);
        self
    }

    pub fn with_local_vars<I>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (&'a String, &'a Option<String>)>,
    {
        self.local_vars.extend(vars);
        self
    }

    pub fn load(
        &self,
        content: impl AsRef<str>,
        path: impl AsRef<Path>,
    ) -> miette::Result<FxHashMap<String, Option<String>>> {
        let mut vars = FxHashMap::default();

        for (i, line) in content.as_ref().lines().enumerate() {
            let line_no = i + 1;

            let Some((key, value, quote)) =
                self.parse_line(line)
                    .map_err(|error| DotEnvError::ParseFailure {
                        line: line_no,
                        message: error.to_string(),
                        path: path.as_ref().to_path_buf(),
                    })?
            else {
                continue;
            };

            let value = if quote == QuoteStyle::Single {
                value
            } else {
                self.substitute_value(&key, &value, &vars)
            };

            vars.insert(key, Some(value));
        }

        Ok(vars)
    }

    pub fn load_file(
        &self,
        path: impl AsRef<Path>,
    ) -> miette::Result<FxHashMap<String, Option<String>>> {
        self.load(fs::read_file(path.as_ref())?, path.as_ref())
    }

    pub fn parse_line(&self, line: &str) -> miette::Result<Option<(String, String, QuoteStyle)>> {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(None);
        }

        // Handle export prefix
        let mut line_content = if let Some(stripped) = trimmed.strip_prefix("export ") {
            stripped
        } else {
            trimmed
        };

        // Handle trailing comment
        if let Some(index) = line_content.rfind(" #") {
            let quote_indexes = get_quote_indices(line_content);

            if quote_indexes.is_none_or(|(_l, r)| index > r) {
                line_content = &line_content[0..index];
            }
        }

        // Find the = separator
        let Some(eq_pos) = line_content.find('=') else {
            return Err(DotEnvError::MissingAssignment.into());
        };

        let key = line_content[..eq_pos].trim();
        let value_part = line_content[eq_pos + 1..].trim();

        let key = self.parse_key(key)?;
        let (value, quote) = self.parse_value(value_part)?;

        Ok(Some((key, value, quote)))
    }

    pub fn parse_key(&self, key: &str) -> miette::Result<String> {
        if key.is_empty() {
            return Err(DotEnvError::EmptyKey.into());
        }

        let chars = key.chars();

        for (i, ch) in chars.enumerate() {
            if i == 0 && !ch.is_alphabetic() && ch != '_' {
                return Err(DotEnvError::InvalidKeyPrefix {
                    key: key.to_owned(),
                }
                .into());
            }

            if !ch.is_alphanumeric() && ch != '_' {
                return Err(DotEnvError::InvalidKey {
                    key: key.to_owned(),
                }
                .into());
            }
        }

        Ok(key.to_owned())
    }

    pub fn parse_value(&self, value: &str) -> miette::Result<(String, QuoteStyle)> {
        let value = value.trim();

        // Handle single quotes (no expansion)
        if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
            return Ok((value[1..value.len() - 1].to_string(), QuoteStyle::Single));
        }

        // Handle double quotes (with expansion)
        if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
            return Ok((
                self.unescape(&value[1..value.len() - 1]),
                QuoteStyle::Double,
            ));
        }

        Ok((value.to_string(), QuoteStyle::Unquoted))
    }

    pub fn substitute_value(
        &self,
        key: &str,
        value: &str,
        env: &FxHashMap<String, Option<String>>,
    ) -> String {
        let mut substitutor = EnvSubstitutor::default().with_local_vars(env);

        if let Some(vars) = &self.global_vars {
            substitutor = substitutor.with_global_vars(vars);
        }

        substitutor.substitute_with_key(key, value)
    }

    fn unescape(&self, value: &str) -> String {
        let mut result = String::new();
        let mut chars = value.chars();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(next_ch) = chars.next() {
                    match next_ch {
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        _ => {
                            result.push('\\');
                            result.push(next_ch);
                        }
                    }
                } else {
                    result.push('\\');
                }
            } else {
                result.push(ch);
            }
        }

        result
    }
}

fn get_quote_indices(value: &str) -> Option<(usize, usize)> {
    if let Some(l) = value.find("'")
        && let Some(r) = value.rfind("'")
        && l != r
    {
        return Some((l, r));
    }

    if let Some(l) = value.find("\"")
        && let Some(r) = value.rfind("\"")
        && l != r
    {
        return Some((l, r));
    }

    None
}
