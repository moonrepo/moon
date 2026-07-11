use crate::target_error::TargetError;
use crate::target_scope::{TargetProjectScope, TargetTaskScope};
use compact_str::CompactString;
use moon_common::{ID_CHARS, ID_SYMBOLS, Style, Stylize, color};
use regex::Regex;
use schematic::{Schema, SchemaBuilder, Schematic};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::sync::LazyLock;
use std::{cmp::Ordering, fmt};
use tracing::instrument;

// The @ is to support npm package scopes!
pub static TARGET_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(?P<project>(?:[A-Za-z@#_]{{1}}[{ID_CHARS}{ID_SYMBOLS}]*|\^(?:build|development|dev|peer|production|prod)?|~))?:(?P<task>#?[{ID_CHARS}{ID_SYMBOLS}]+)$"
    ))
    .unwrap()
});

fn map_error(value: &str) -> TargetError {
    TargetError::InvalidFormat(value.to_owned())
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Target {
    pub id: CompactString,

    #[doc(hidden)]
    pub project: TargetProjectScope,

    #[doc(hidden)]
    pub task: TargetTaskScope,

    // Index of the `:` separator, used for fast slicing
    #[doc(hidden)]
    pub index: u8,
}

impl Target {
    pub fn new<P, T>(project_id: P, task_id: T) -> miette::Result<Target>
    where
        P: AsRef<str>,
        T: AsRef<str>,
    {
        let project_id = project_id.as_ref();
        let task_id = task_id.as_ref();
        let id = format!("{project_id}:{task_id}");

        Ok(Target {
            index: project_id.len() as u8,
            project: TargetProjectScope::parse(project_id).map_err(|_| map_error(&id))?,
            task: TargetTaskScope::parse(task_id).map_err(|_| map_error(&id))?,
            id: CompactString::new(id),
        })
    }

    pub fn new_self<T>(task_id: T) -> miette::Result<Target>
    where
        T: AsRef<str>,
    {
        let task_id = task_id.as_ref();
        let id = format!("~:{task_id}");

        Ok(Target {
            index: 1,
            project: TargetProjectScope::OwnSelf,
            task: TargetTaskScope::parse(task_id).map_err(|_| map_error(&id))?,
            id: CompactString::new(id),
        })
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

        let project_id = matches.name("project").map(|m| m.as_str()).unwrap_or("");
        let task_id = matches.name("task").expect("Task ID required.").as_str();

        Self::new(project_id, task_id)
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
        if matches!(&self.project, TargetProjectScope::All)
            && let Ok(inner_id) = self.get_task_id()
        {
            return if let Some(id) = task_id.strip_prefix(':') {
                inner_id == id
            } else {
                inner_id == task_id
            };
        }

        false
    }

    pub fn is_fully_qualified(&self) -> bool {
        matches!(
            self.project,
            TargetProjectScope::Id | TargetProjectScope::OwnSelf
        ) && matches!(self.task, TargetTaskScope::Id)
    }

    pub fn get_project_id(&self) -> miette::Result<&str> {
        let (scope, value) = self.get_project_scope();

        match scope {
            TargetProjectScope::Id => Ok(value),
            _ => Err(TargetError::ProjectScopeRequired(self.id.to_string()).into()),
        }
    }

    pub fn get_project_tag(&self) -> Option<&str> {
        let (scope, value) = self.get_project_scope();

        if let TargetProjectScope::Tag = scope {
            Some(value)
        } else {
            None
        }
    }

    pub fn get_project_scope(&self) -> (TargetProjectScope, &str) {
        let project = &self.id[0..self.index as usize];

        match &self.project {
            TargetProjectScope::DepsOf(scope) => (
                TargetProjectScope::DepsOf(*scope),
                project.trim_start_matches('^'),
            ),
            TargetProjectScope::Id => (TargetProjectScope::Id, project),
            TargetProjectScope::Tag => (TargetProjectScope::Tag, project.trim_start_matches('#')),
            _ => (self.project.clone(), ""),
        }
    }

    pub fn get_task_id(&self) -> miette::Result<&str> {
        let (scope, value) = self.get_task_scope();

        match scope {
            TargetTaskScope::Id => Ok(value),
            _ => Err(TargetError::TaskScopeRequired(self.id.to_string()).into()),
        }
    }

    pub fn get_task_tag(&self) -> Option<&str> {
        let (scope, value) = self.get_task_scope();

        if let TargetTaskScope::Tag = scope {
            Some(value)
        } else {
            None
        }
    }

    pub fn get_task_scope(&self) -> (TargetTaskScope, &str) {
        let value = &self.id[self.index as usize + 1..];

        match value.strip_prefix('#') {
            Some(tag) => (TargetTaskScope::Tag, tag),
            None => (TargetTaskScope::Id, value),
        }
    }
}

impl Default for Target {
    fn default() -> Self {
        Target {
            id: "~:unknown".into(),
            project: TargetProjectScope::OwnSelf,
            task: TargetTaskScope::Id,
            index: 1,
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
