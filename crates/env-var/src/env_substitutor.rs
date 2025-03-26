use crate::global_bag::GlobalEnvBag;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::LazyLock;

// $ENV_VAR, ${ENV_VAR}
pub static ENV_VAR_SUBSTITUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(?:\\$\\{(?P<name1>[A-Z0-9_]+)(?P<flag1>[!?]{1})?\\})|(?:\\$(?P<name2>[A-Z0-9_]+)(?P<flag2>[!?]{1})?)").unwrap()
});

#[derive(Default)]
pub struct EnvSubstitutor<'bag> {
    pub replaced: FxHashSet<String>,

    global_vars: Option<&'bag GlobalEnvBag>,
    local_vars: Option<&'bag FxHashMap<String, String>>,
}

impl<'bag> EnvSubstitutor<'bag> {
    pub fn new() -> Self {
        Self::default().with_global_vars(GlobalEnvBag::instance())
    }

    pub fn without_global_vars(mut self) -> Self {
        self.global_vars = None;
        self
    }

    pub fn with_global_vars(mut self, vars: &'bag GlobalEnvBag) -> Self {
        self.global_vars = Some(vars);
        self
    }

    pub fn with_local_vars(mut self, vars: &'bag FxHashMap<String, String>) -> Self {
        self.local_vars = Some(vars);
        self
    }

    pub fn substitute(&mut self, value: impl AsRef<str>) -> String {
        self.do_substitute(value, None)
    }

    pub fn substitute_with_key(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> String {
        self.do_substitute(value, Some(key.as_ref()))
    }

    pub fn substitute_all(
        &mut self,
        vars: &FxHashMap<String, String>,
    ) -> FxHashMap<String, String> {
        let mut map = FxHashMap::default();

        for (key, value) in vars {
            map.insert(key.into(), self.substitute_with_key(key, value));
        }

        map
    }

    fn do_substitute(&mut self, value: impl AsRef<str>, base_key: Option<&str>) -> String {
        let value = value.as_ref();

        if !value.contains('$') {
            self.replaced.clear();

            return value.to_owned();
        }

        let global_vars = self.global_vars;
        let local_vars = self.local_vars;
        let mut substituted = FxHashSet::default();

        let result = ENV_VAR_SUBSTITUTE.replace_all(value, |caps: &regex::Captures| {
            let Some(name) = caps
                .name("name1")
                .or_else(|| caps.name("name2"))
                .map(|cap| cap.as_str())
            else {
                return String::new();
            };

            let flag = caps
                .name("flag1")
                .or_else(|| caps.name("flag2"))
                .map(|cap| cap.as_str());

            // If the variable is referencing itself, don't pull
            // from the local map, and instead only pull from the
            // system environment. Otherwise we hit recursion!
            let mut get_replacement_value = || {
                substituted.insert(name.to_owned());

                if base_key.is_none() || base_key.is_some_and(|base_name| base_name != name) {
                    if let Some(value) = local_vars.and_then(|bag| bag.get(name)) {
                        return Some(value.to_owned());
                    }
                }

                global_vars.and_then(|bag| bag.get(name))
            };

            match flag {
                // Don't substitute
                Some("!") => format!("${name}"),
                // Substitute with empty string when missing
                Some("?") => get_replacement_value().unwrap_or_default(),
                // Substitute with self when missing
                _ => get_replacement_value()
                    .unwrap_or_else(|| caps.get(0).unwrap().as_str().to_owned()),
            }
        });

        self.replaced = substituted;

        result.to_string()
    }
}
