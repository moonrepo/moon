use crate::dotenv_error::DotEnvError;
use crate::global_bag::GlobalEnvBag;
use crate::{ENV_VAR, ENV_VAR_BRACKETS};
use regex::Captures;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::borrow::Cow;
use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum QuoteStyle {
    Single,
    Double,
    Unquoted,
}

#[derive(Default)]
pub struct DotEnv<'a> {
    command_vars: FxHashMap<&'a OsString, &'a Option<OsString>>,
}

impl<'a> DotEnv<'a> {
    pub fn with_command_vars<I>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (&'a OsString, &'a Option<OsString>)>,
    {
        self.command_vars.extend(vars.into_iter());
        self
    }

    pub fn load(
        &self,
        content: impl AsRef<str>,
        path: impl AsRef<Path>,
    ) -> miette::Result<FxHashMap<String, String>> {
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

            vars.insert(
                key,
                if quote == QuoteStyle::Single {
                    value
                } else {
                    self.expand_value(value, &vars)
                },
            );
        }

        Ok(vars)
    }

    pub fn load_file(&self, path: impl AsRef<Path>) -> miette::Result<FxHashMap<String, String>> {
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

        // Validate key
        if key.is_empty() {
            return Err(DotEnvError::EmptyKey.into());
        }

        // Parse value (handle quotes)
        let (value, quote) = self.parse_value(value_part)?;

        Ok(Some((key.to_string(), value, quote)))
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

    // https://dotenvx.com/docs/env-file#interpolation
    pub fn expand_value(&self, value: String, env: &FxHashMap<String, String>) -> String {
        let bag = GlobalEnvBag::instance();

        let get_expanded_value = |key: &str| {
            // Command/task first as they take predence over .env files
            if let Some(Some(val)) = self.command_vars.get(&OsString::from(key)) {
                return val.to_string_lossy();
            }

            // Then check the current .env file
            if let Some(val) = env.get(key) {
                return Cow::Borrowed(val);
            }

            // Otherwise the global process last
            if let Some(val) = bag.get(key) {
                return Cow::Owned(val);
            }

            Cow::Owned(String::new())
        };

        // Expand brackets first
        let value = ENV_VAR_BRACKETS.replace_all(&value, |caps: &Captures| {
            let Some(name) = caps.name("name").map(|cap| cap.as_str()) else {
                return String::new();
            };

            let fallback = caps
                .name("fallback")
                .map(|cap| cap.as_str())
                .unwrap_or_default();

            match caps.name("flag").map(|cap| cap.as_str()) {
                // Don't expand
                Some("!") => caps.get(0).unwrap().as_str().to_owned(),
                // Only expand if not empty
                Some("?") => {
                    let value = get_expanded_value(name);

                    if value.is_empty() {
                        caps.get(0).unwrap().as_str().to_owned()
                    } else {
                        value.to_string()
                    }
                }
                // Expand with default/alternate
                Some(":") => {
                    let value = get_expanded_value(name);

                    if let Some(def) = fallback.strip_prefix('-') {
                        if value.is_empty() {
                            def.to_owned()
                        } else {
                            value.to_string()
                        }
                    } else if let Some(alt) = fallback.strip_prefix('+') {
                        if value.is_empty() {
                            value.to_string()
                        } else {
                            alt.to_owned()
                        }
                    } else {
                        value.to_string()
                    }
                }
                // Expand
                _ => get_expanded_value(name).to_string(),
            }
        });

        // Expand non-brackets last
        let value = ENV_VAR.replace_all(&value, |caps: &Captures| {
            match caps.name("name").map(|cap| cap.as_str()) {
                Some(name) => get_expanded_value(name).to_string(),
                None => String::new(),
            }
        });

        value.to_string()
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
