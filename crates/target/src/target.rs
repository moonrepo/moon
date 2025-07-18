use crate::target_error::TargetError;
use crate::target_scope::TargetScope;
use compact_str::CompactString;
use moon_common::{ID_CHARS, Id, Style, Stylize, color};
use regex::Regex;
use schematic::{Schema, SchemaBuilder, Schematic};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::sync::LazyLock;
use std::{cmp::Ordering, fmt};
use tracing::instrument;

// The @ is to support npm package scopes!
pub static TARGET_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(?P<scope>(?:[A-Za-z@#_]{{1}}{ID_CHARS}|\^|~))?:(?P<task>{ID_CHARS})$"
    ))
    .unwrap()
});

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Target {
    pub id: CompactString,
    pub scope: TargetScope,
    pub task_id: Id,
}

impl Target {
    pub fn new<S, T>(scope_id: S, task_id: T) -> miette::Result<Target>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        let scope_id = scope_id.as_ref();
        let task_id = task_id.as_ref();

        let handle_error = |_| TargetError::InvalidFormat(format!("{scope_id}:{task_id}"));
        let scope = TargetScope::Project(Id::new(scope_id).map_err(handle_error)?);

        Ok(Target {
            id: CompactString::new(Target::format(&scope, task_id)),
            scope,
            task_id: Id::new(task_id).map_err(handle_error)?,
        })
    }

    pub fn new_self<T>(task_id: T) -> miette::Result<Target>
    where
        T: AsRef<str>,
    {
        let task_id = task_id.as_ref();

        Ok(Target {
            id: CompactString::new(Target::format(TargetScope::OwnSelf, task_id)),
            scope: TargetScope::OwnSelf,
            task_id: Id::new(task_id)
                .map_err(|_| TargetError::InvalidFormat(format!("~:{task_id}")))?,
        })
    }

    pub fn format<S, T>(scope: S, task: T) -> String
    where
        S: AsRef<TargetScope>,
        T: AsRef<str>,
    {
        format!("{}:{}", scope.as_ref(), task.as_ref())
    }

    #[instrument(name = "parse_target")]
    pub fn parse(target_id: &str) -> miette::Result<Target> {
        if target_id == ":" {
            return Err(TargetError::TooWild.into());
        }

        if !target_id.contains(':') {
            return Target::new_self(target_id);
        }

        let Some(matches) = TARGET_PATTERN.captures(target_id) else {
            return Err(TargetError::InvalidFormat(target_id.to_owned()).into());
        };

        let scope = match matches.name("scope") {
            Some(value) => match value.as_str() {
                "" => TargetScope::All,
                "^" => TargetScope::Deps,
                "~" => TargetScope::OwnSelf,
                id => {
                    if let Some(tag) = id.strip_prefix('#') {
                        TargetScope::Tag(Id::raw(tag))
                    } else {
                        TargetScope::Project(Id::raw(id))
                    }
                }
            },
            None => TargetScope::All,
        };

        let task_id = Id::new(matches.name("task").unwrap().as_str())
            .map_err(|_| TargetError::InvalidFormat(target_id.to_owned()))?;

        Ok(Target {
            id: CompactString::new(target_id),
            scope,
            task_id,
        })
    }

    pub fn parse_strict(target_id: &str) -> miette::Result<Target> {
        if !target_id.contains(':') {
            return Err(TargetError::ProjectScopeRequired(target_id.into()).into());
        }

        Self::parse(target_id)
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }

    pub fn to_prefix(&self, width: Option<usize>) -> String {
        let prefix = self.as_str();

        let label = if let Some(width) = width {
            format!("{prefix: >width$}")
        } else {
            prefix.to_owned()
        };

        if color::no_color() {
            format!("{label} | ")
        } else {
            format!("{} {} ", color::log_target(label), color::muted("|"))
        }
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

    pub fn get_project_id(&self) -> miette::Result<&Id> {
        match &self.scope {
            TargetScope::Project(id) => Ok(id),
            _ => Err(TargetError::ProjectScopeRequired(self.id.to_string()).into()),
        }
    }

    pub fn get_tag_id(&self) -> Option<&Id> {
        match &self.scope {
            TargetScope::Tag(id) => Some(id),
            _ => None,
        }
    }
}

impl Default for Target {
    fn default() -> Self {
        Target {
            id: "~:unknown".into(),
            scope: TargetScope::OwnSelf,
            task_id: Id::raw("unknown"),
        }
    }
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl Stylize for Target {
    fn style(&self, style: Style) -> String {
        self.to_string().style(style)
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

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Target::parse(&String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl Serialize for Target {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.id)
    }
}

impl Schematic for Target {
    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.string_default()
    }
}

// This is only used by tests!

impl From<&str> for Target {
    fn from(value: &str) -> Self {
        Target::parse(value).unwrap()
    }
}
