use crate::global_bag::GlobalEnvBag;
use crate::{ENV_VAR, ENV_VAR_BRACKETS};
use indexmap::IndexMap;
use regex::Captures;
use rustc_hash::FxHashSet;
use std::borrow::Cow;

#[derive(Default)]
pub struct EnvSubstitutor<'bag> {
    pub replaced: FxHashSet<String>,

    global_vars: Option<&'bag GlobalEnvBag>,
    local_vars: IndexMap<&'bag String, &'bag Option<String>>,
}

impl<'bag> EnvSubstitutor<'bag> {
    pub fn with_global_vars(mut self, vars: &'bag GlobalEnvBag) -> Self {
        self.global_vars = Some(vars);
        self
    }

    pub fn with_local_vars<I>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (&'bag String, &'bag Option<String>)>,
    {
        self.local_vars.extend(vars);
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
        vars: &IndexMap<String, Option<String>>,
    ) -> IndexMap<String, Option<String>> {
        let mut map = IndexMap::default();

        for (key, value) in vars {
            map.insert(
                key.into(),
                value.as_ref().map(|val| self.substitute_with_key(key, val)),
            );
        }

        map
    }

    // https://dotenvx.com/docs/env-file#interpolation
    fn do_substitute(&mut self, value: impl AsRef<str>, parent_key: Option<&str>) -> String {
        let value = value.as_ref();

        if !value.contains('$') {
            self.replaced.clear();

            return value.to_owned();
        }

        let mut substituted = FxHashSet::default();

        // Expand non-brackets first
        let value = ENV_VAR.replace_all(value, |caps: &Captures| {
            match caps.name("name").map(|cap| cap.as_str()) {
                Some(key) => {
                    substituted.insert(key.to_owned());
                    self.get_replacement_value(key, parent_key).to_string()
                }
                None => String::new(),
            }
        });

        // Expand brackets last
        let value = ENV_VAR_BRACKETS.replace_all(&value, |caps: &Captures| {
            let Some(key) = caps.name("name").map(|cap| cap.as_str()) else {
                return String::new();
            };

            substituted.insert(key.to_owned());

            let namespace = caps
                .name("namespace")
                .map(|cap| cap.as_str())
                .unwrap_or_default();

            let fallback = caps
                .name("fallback")
                .map(|cap| cap.as_str())
                .unwrap_or_default();

            match caps.name("flag").map(|cap| cap.as_str()) {
                // Don't expand
                Some("!") => {
                    substituted.remove(key);
                    self.get_token_value(namespace, key)
                }
                // Only expand if not empty
                Some("?") => {
                    let value = self.get_replacement_value(key, parent_key);

                    if value.is_empty() {
                        self.get_token_value(namespace, key)
                    } else {
                        value.to_string()
                    }
                }
                // Expand with default if empty
                Some(":" | ":-" | "-") => {
                    let value = self.get_replacement_value(key, parent_key);

                    if value.is_empty() {
                        fallback.to_owned()
                    } else {
                        value.to_string()
                    }
                }
                // Expand with alternate if not empty
                Some(":+" | "+") => {
                    let value = self.get_replacement_value(key, parent_key);

                    if value.is_empty() {
                        value.to_string()
                    } else {
                        fallback.to_owned()
                    }
                }
                // Expand
                _ => self.get_replacement_value(key, parent_key).to_string(),
            }
        });

        self.replaced = substituted;

        value.to_string()
    }

    pub fn get_token_value(&self, namespace: &str, key: &str) -> String {
        // Ion must always be wrapped in brackets!
        if namespace == "env::" {
            format!("${{{namespace}{key}}}")
        } else {
            format!("${namespace}{key}")
        }
    }

    // If the variable is referencing itself, don't pull
    // from the local map, and instead only pull from the
    // globals. Otherwise we hit recursion!
    pub fn get_replacement_value(&self, key: &str, parent_key: Option<&str>) -> Cow<'_, str> {
        let is_self = parent_key.is_some_and(|k| k == key);

        // Then check the locals
        if !is_self && let Some(Some(val)) = self.local_vars.get(&String::from(key)) {
            return Cow::Borrowed(val);
        }

        // Otherwise the globals
        if let Some(bag) = &self.global_vars
            && let Some(val) = bag.get(key)
        {
            return Cow::Owned(val);
        }

        Cow::Owned(String::new())
    }
}
