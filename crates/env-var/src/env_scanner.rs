use crate::env_substitutor::{ENV_VAR_SUBSTITUTE, ENV_VAR_SUBSTITUTE_BRACKETS};
use rustc_hash::FxHashSet;

#[derive(Default)]
pub struct EnvScanner {
    pub found: FxHashSet<String>,
}

impl EnvScanner {
    pub fn scan(&mut self, value: impl AsRef<str>) -> String {
        self.do_scan(value)
    }

    fn do_scan(&mut self, value: impl AsRef<str>) -> String {
        let value = value.as_ref();

        if !value.contains('$') {
            self.found.clear();

            return value.to_owned();
        }

        let mut found = FxHashSet::default();

        ENV_VAR_SUBSTITUTE.replace_all(value, |caps: &regex::Captures| {
            self.do_replace(caps, &mut found)
        });

        ENV_VAR_SUBSTITUTE_BRACKETS.replace_all(value, |caps: &regex::Captures| {
            self.do_replace(caps, &mut found)
        });

        self.found = found;

        value.into()
    }

    fn do_replace(&self, caps: &regex::Captures, found: &mut FxHashSet<String>) -> String {
        if let Some(name) = caps.name("name").map(|cap| cap.as_str()) {
            found.insert(name.to_owned());
        }

        String::new()
    }
}
