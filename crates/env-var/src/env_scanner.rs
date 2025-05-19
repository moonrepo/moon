use crate::env_substitutor::ENV_VAR_SUBSTITUTE;
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

        let result = ENV_VAR_SUBSTITUTE.replace_all(value, |caps: &regex::Captures| {
            let Some(name) = caps
                .name("name1")
                .or_else(|| caps.name("name2"))
                .map(|cap| cap.as_str())
            else {
                return String::new();
            };

            found.insert(name.to_owned());

            format!("${name}")
        });

        self.found = found;

        result.to_string()
    }
}
