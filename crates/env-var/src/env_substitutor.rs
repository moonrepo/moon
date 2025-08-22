use crate::global_bag::GlobalEnvBag;
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::LazyLock;

// $E: = Elvish
// $env: = PowerShell
// $env:: = Ion
// $env. = Nu
// $ENV. = Murex

// $ENV_VAR
pub static ENV_VAR_SUBSTITUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?:\\$(?P<namespace>E:|env::|env:|env.|ENV.)?(?P<name>[A-Z0-9_]+)(?P<flag>[!?]{1})?)",
    )
    .unwrap()
});

// ${ENV_VAR}
pub static ENV_VAR_SUBSTITUTE_BRACKETS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?:\\$\\{(?P<namespace>E:|env::|env:|env.|ENV.)?(?P<name>[A-Z0-9_]+)(?P<flag>[!?:]{1})?(?P<fallback>[^}]*)?\\})",
    )
    .unwrap()
});

pub fn contains_env_var(value: impl AsRef<str>) -> bool {
    ENV_VAR_SUBSTITUTE.is_match(value.as_ref())
        || ENV_VAR_SUBSTITUTE_BRACKETS.is_match(value.as_ref())
}

pub fn rebuild_env_var(caps: &Captures) -> String {
    let namespace = caps
        .name("namespace")
        .map(|cap| cap.as_str())
        .unwrap_or_default();
    let name = caps.name("name").map(|cap| cap.as_str()).unwrap();

    // Ion must always be wrapped in brackets!
    if namespace == "env::" {
        format!("${{{namespace}{name}}}")
    } else {
        format!("${namespace}{name}")
    }
}

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

        let mut substituted = FxHashSet::default();

        let result = ENV_VAR_SUBSTITUTE.replace_all(value, |caps: &Captures| {
            self.do_replace(caps, base_key, &mut substituted)
        });

        let result = ENV_VAR_SUBSTITUTE_BRACKETS.replace_all(&result, |caps: &Captures| {
            self.do_replace(caps, base_key, &mut substituted)
        });

        self.replaced = substituted;

        result.to_string()
    }

    fn do_replace(
        &self,
        caps: &Captures,
        base_key: Option<&str>,
        substituted: &mut FxHashSet<String>,
    ) -> String {
        let haystack = caps.get(0).unwrap().as_str();

        let Some(name) = caps.name("name").map(|cap| cap.as_str()) else {
            return String::new();
        };

        let global_vars = self.global_vars;
        let local_vars = self.local_vars;
        let flag = caps.name("flag").map(|cap| cap.as_str());

        // If the variable is referencing itself, don't pull
        // from the local map, and instead only pull from the
        // system environment. Otherwise we hit recursion!
        let mut get_replacement_value = || {
            substituted.insert(name.to_owned());

            if (base_key.is_none() || base_key.is_some_and(|base_name| base_name != name))
                && let Some(value) = local_vars.and_then(|bag| bag.get(name))
            {
                return Some(value.to_owned());
            }

            global_vars.and_then(|bag| bag.get(name))
        };

        match flag {
            // Don't substitute
            Some("!") => rebuild_env_var(caps),
            // Substitute with provided fallback
            Some(":") => caps
                .name("fallback")
                .map(|cap| cap.as_str())
                .unwrap_or_default()
                .to_string(),
            // Substitute with empty string when missing
            Some("?") => get_replacement_value().unwrap_or_default(),
            // Substitute with self when missing
            _ => get_replacement_value().unwrap_or_else(|| haystack.to_owned()),
        }
    }
}
