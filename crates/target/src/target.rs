use crate::dep_scope::DependencyScope;
use crate::target_error::TargetError;
use crate::target_scope::{TargetProjectScope, TargetTaskScope};
use compact_str::CompactString;
use moon_common::{ID_CHARS, ID_SYMBOLS, Id, Style, Stylize, color};
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

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Target {
    pub id: CompactString,
    pub project: TargetProjectScope,
    pub task: TargetTaskScope,
}

impl Target {
    pub fn new<T>(project: TargetProjectScope, task_id: T) -> miette::Result<Target>
    where
        T: AsRef<str>,
    {
        let task_id = task_id.as_ref();
        let id = Target::format(&project, task_id);

        Ok(Target {
            task: TargetTaskScope::parse(task_id)?,
            id: CompactString::new(id),
            project,
        })
    }

    pub fn new_deps<T>(task_id: T) -> miette::Result<Target>
    where
        T: AsRef<str>,
    {
        Self::new(TargetProjectScope::Deps, task_id)
    }

    pub fn new_deps_of<T>(deps: DependencyScope, task_id: T) -> miette::Result<Target>
    where
        T: AsRef<str>,
    {
        Self::new(TargetProjectScope::DepsOf(deps), task_id)
    }

    pub fn new_project<S, T>(project_id: S, task_id: T) -> miette::Result<Target>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        let project_id = project_id.as_ref();
        let task_id = task_id.as_ref();

        Self::new(
            TargetProjectScope::Id(
                Id::new(project_id)
                    .map_err(|_| TargetError::InvalidFormat(format!("{project_id}:{task_id}")))?,
            ),
            task_id,
        )
    }

    pub fn new_project_tag<S, T>(tag_id: S, task_id: T) -> miette::Result<Target>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        let tag_id = tag_id.as_ref();
        let task_id = task_id.as_ref();

        Self::new(
            TargetProjectScope::Tag(
                Id::new(tag_id)
                    .map_err(|_| TargetError::InvalidFormat(format!("#{tag_id}:{task_id}")))?,
            ),
            task_id,
        )
    }

    pub fn new_self<T>(task_id: T) -> miette::Result<Target>
    where
        T: AsRef<str>,
    {
        Self::new(TargetProjectScope::OwnSelf, task_id)
    }

    pub fn format<S, T>(project: S, task: T) -> String
    where
        S: AsRef<TargetProjectScope>,
        T: AsRef<str>,
    {
        format!("{}:{}", project.as_ref(), task.as_ref())
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

        let project = match matches.name("project") {
            Some(value) => TargetProjectScope::parse(value.as_str())?,
            None => TargetProjectScope::All,
        };

        let task_id = matches.name("task").expect("Task ID required.").as_str();

        Self::new(project, task_id)
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

    pub fn get_project_id(&self) -> miette::Result<&Id> {
        match &self.project {
            TargetProjectScope::Id(id) => Ok(id),
            _ => Err(TargetError::ProjectScopeRequired(self.id.to_string()).into()),
        }
    }

    pub fn get_project_tag_id(&self) -> Option<&Id> {
        match &self.project {
            TargetProjectScope::Tag(id) => Some(id),
            _ => None,
        }
    }

    pub fn get_task_id(&self) -> miette::Result<&Id> {
        match &self.task {
            TargetTaskScope::Id(id) => Ok(id),
            _ => Err(TargetError::TaskScopeRequired(self.id.to_string()).into()),
        }
    }

    pub fn get_task_tag_id(&self) -> Option<&Id> {
        match &self.task {
            TargetTaskScope::Tag(id) => Some(id),
            _ => None,
        }
    }
}

impl Default for Target {
    fn default() -> Self {
        Target {
            id: "~:unknown".into(),
            project: TargetProjectScope::OwnSelf,
            task: TargetTaskScope::Id(Id::raw("unknown")),
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
