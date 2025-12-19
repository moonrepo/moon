use crate::{ENV_VAR, ENV_VAR_BRACKETS};
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

            return value.into();
        }

        let mut found = FxHashSet::default();

        for caps in ENV_VAR.captures_iter(value) {
            self.do_capture(caps, &mut found);
        }

        for caps in ENV_VAR_BRACKETS.captures_iter(value) {
            self.do_capture(caps, &mut found);
        }

        self.found = found;

        value.into()
    }

    fn do_capture(&self, caps: regex::Captures, found: &mut FxHashSet<String>) {
        if let Some(name) = caps.name("name").map(|cap| cap.as_str()) {
            found.insert(name.to_owned());
        }
    }
}
