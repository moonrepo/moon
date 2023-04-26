use crate::errors::TargetError;
use crate::target_scope::TargetScope;
use moon_common::{Id, ID_CHARS};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{self, Display},
};

// The project scope supports `@` because of Node.js packages,
// but we don't want to support it in regular IDs!
pub static TARGET_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^(?P<scope>(?:[A-Za-z@#]{{1}}{chars}|\^|~))?:(?P<task>{chars})$",
        chars = ID_CHARS
    ))
    .unwrap()
});

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Target {
    pub id: String,
    pub scope: TargetScope,
    pub scope_id: Option<Id>,
    pub task_id: Id,
}

impl Target {
    pub fn new<S, T>(project_id: S, task_id: T) -> Result<Target, TargetError>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        let project_id = project_id.as_ref();
        let task_id = task_id.as_ref();
        let scope = TargetScope::Project(Id::new(project_id)?);

        Ok(Target {
            id: Target::format(&scope, task_id)?,
            scope,
            scope_id: Some(Id::raw(project_id)),
            task_id: Id::new(task_id)?,
        })
    }

    pub fn new_self<T>(task_id: T) -> Result<Target, TargetError>
    where
        T: AsRef<str>,
    {
        let task_id = task_id.as_ref();

        Ok(Target {
            id: Target::format(TargetScope::OwnSelf, task_id)?,
            scope: TargetScope::OwnSelf,
            scope_id: None,
            task_id: Id::new(task_id)?,
        })
    }

    pub fn format<S, T>(scope: S, task: T) -> Result<String, TargetError>
    where
        S: AsRef<TargetScope>,
        T: AsRef<str>,
    {
        Ok(format!("{}:{}", scope.as_ref(), task.as_ref()))
    }

    pub fn parse(target_id: &str) -> Result<Target, TargetError> {
        if target_id == ":" {
            return Err(TargetError::TooWild);
        }

        let Some(matches) = TARGET_PATTERN.captures(target_id) else {
            return Err(TargetError::InvalidFormat(target_id.to_owned()));
        };

        let mut scope_id = None;
        let scope = match matches.name("scope") {
            Some(value) => match value.as_str() {
                "" => TargetScope::All,
                "^" => TargetScope::Deps,
                "~" => TargetScope::OwnSelf,
                id => {
                    if let Some(tag) = id.strip_prefix('#') {
                        scope_id = Some(Id::raw(tag));
                        TargetScope::Tag(Id::raw(tag))
                    } else {
                        scope_id = Some(Id::raw(id));
                        TargetScope::Project(Id::raw(id))
                    }
                }
            },
            None => TargetScope::All,
        };

        let task_id = Id::raw(matches.name("task").unwrap().as_str());

        Ok(Target {
            id: target_id.to_owned(),
            scope,
            scope_id,
            task_id,
        })
    }

    pub fn is_all_task(&self, task_id: &str) -> bool {
        if matches!(&self.scope, TargetScope::All) {
            return if let Some(id) = task_id.strip_prefix(':') {
                self.task_id == id
            } else {
                self.task_id == task_id
            };
        }

        false
    }
}

impl Default for Target {
    fn default() -> Self {
        Target {
            id: "~:unknown".into(),
            scope: TargetScope::OwnSelf,
            scope_id: None,
            task_id: Id::raw("unknown"),
        }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl AsRef<Target> for Target {
    fn as_ref(&self) -> &Target {
        self
    }
}

impl AsRef<str> for Target {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl PartialOrd for Target {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Target {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

// These traits are for converting targets within configuration
// into the `Target` object instead of strings.

impl TryFrom<String> for Target {
    type Error = TargetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Target::parse(&value)
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for Target {
    fn into(self) -> String {
        self.id
    }
}
