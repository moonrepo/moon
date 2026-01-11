use moon_common::cacheable;
use std::ops::Deref;

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(into = "String", from = "String")]
    pub struct TaskArg {
        // For use in sub-shells
        pub quoted_value: Option<String>,

        // For use in processes
        pub value: String, // unquoted
    }
);

impl TaskArg {
    pub fn new(base_value: impl AsRef<str>) -> Self {
        let base_value = base_value.as_ref();
        let mut is_quoted = false;
        let mut quoted_value = None;
        let mut value = String::new();

        // Keep in sync with starbase!
        for (start, end) in [
            ("\"", "\""),
            ("'", "'"),
            ("$\"", "\""),
            ("$'", "'"),
            ("%(", ")"),
            ("r#'", "'#"),
        ] {
            if base_value.starts_with(start) && base_value.ends_with(end) {
                value.push_str(base_value.trim_start_matches(start).trim_end_matches(end));
                quoted_value = Some(base_value.to_owned());
                is_quoted = true;
                break;
            }
        }

        if !is_quoted {
            value.push_str(base_value);
        }

        Self {
            quoted_value,
            value,
        }
    }

    pub fn new_quoted(value: String, quoted_value: String) -> Self {
        Self {
            quoted_value: Some(quoted_value),
            value,
        }
    }

    pub fn new_unquoted(value: String) -> Self {
        Self {
            quoted_value: None,
            value,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn is_quoted(&self) -> bool {
        self.quoted_value.is_some()
    }

    pub fn get_value(&self) -> &str {
        self.quoted_value.as_deref().unwrap_or(&self.value)
    }
}

impl From<TaskArg> for String {
    fn from(arg: TaskArg) -> Self {
        arg.value
    }
}

impl From<String> for TaskArg {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl PartialEq<&str> for TaskArg {
    fn eq(&self, other: &&str) -> bool {
        &self.value == other
    }
}

impl AsRef<str> for TaskArg {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl Deref for TaskArg {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
