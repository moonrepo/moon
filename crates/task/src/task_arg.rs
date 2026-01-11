use moon_common::cacheable;

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
    pub fn new(base_value: String) -> Self {
        let mut quoted_value = None;
        let mut value = String::new();

        // Keep in sync with starbase!
        for (l, r) in [
            ("\"", "\""),
            ("'", "'"),
            ("$\"", "\""),
            ("$'", "'"),
            ("%(", ")"),
            ("r#'", "'#"),
        ] {
            if base_value.starts_with(l) && base_value.ends_with(r) {
                value.push_str(base_value.trim_start_matches(l).trim_end_matches(r));
                quoted_value = Some(base_value);
                break;
            }
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

    pub fn is_quoted(&self) -> bool {
        self.quoted_value.is_some()
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
