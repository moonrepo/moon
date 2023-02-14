use crate::errors::TargetError;
use moon_utils::regex::TARGET_PATTERN;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
// use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum TargetProjectScope {
    All,        // :task
    Deps,       // ^:task
    Id(String), // project:task
    OwnSelf,    // ~:task
}

// impl fmt::Display for TargetProjectScope {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             TargetProjectScope::All => write!(f, ""),
//             TargetProjectScope::Deps => write!(f, "^"),
//             TargetProjectScope::Id(id) => write!(f, "{}", id),
//             TargetProjectScope::Own => write!(f, "~"),
//         }
//     }
// }

// #[derive(Debug, PartialEq)]
// pub enum TargetTask {
//     All,        // project:
//     Id(TaskID), // project:id
// }

// impl fmt::Display for TargetTask {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             TargetTask::All => write!(f, ""),
//             TargetTask::Id(name) => write!(f, "{}", name),
//         }
//     }
// }

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Target {
    pub id: String,

    pub project: TargetProjectScope,

    pub project_id: Option<String>,

    pub task_id: String,
}

impl Target {
    pub fn new(project_id: &str, task_id: &str) -> Result<Target, TargetError> {
        Ok(Target {
            id: Target::format(project_id, task_id)?,
            project: TargetProjectScope::Id(project_id.to_owned()),
            project_id: Some(project_id.to_owned()),
            task_id: task_id.to_owned(),
        })
    }

    pub fn new_self(task_id: &str) -> Result<Target, TargetError> {
        Ok(Target {
            id: Target::format("~", task_id)?,
            project: TargetProjectScope::OwnSelf,
            project_id: None,
            task_id: task_id.to_owned(),
        })
    }

    pub fn format(project_id: &str, task_id: &str) -> Result<String, TargetError> {
        Ok(format!("{project_id}:{task_id}"))
    }

    pub fn parse(target_id: &str) -> Result<Target, TargetError> {
        if target_id == ":" {
            return Err(TargetError::TooWild);
        }

        let Some(matches) = TARGET_PATTERN.captures(target_id) else {
            return Err(TargetError::InvalidFormat(target_id.to_owned()));
        };

        let mut project_id = None;

        let project = match matches.name("project") {
            Some(value) => match value.as_str() {
                "" => TargetProjectScope::All,
                "^" => TargetProjectScope::Deps,
                "~" => TargetProjectScope::OwnSelf,
                id => {
                    project_id = Some(id.to_owned());
                    TargetProjectScope::Id(id.to_owned())
                }
            },
            None => TargetProjectScope::All,
        };

        let task_id = matches.name("task").unwrap().as_str().to_owned();

        // let task = match matches.name("task") {
        //     Some(value) => match value.as_str() {
        //         "" => TargetTask::All,
        //         id => TargetTask::Id(id.to_owned()),
        //     },
        //     None => TargetTask::All,
        // };

        Ok(Target {
            id: target_id.to_owned(),
            project,
            project_id,
            task_id,
        })
    }

    pub fn fail_with(&self, error: TargetError) -> Result<(), TargetError> {
        Err(error)
    }

    pub fn ids(&self) -> Result<(String, String), TargetError> {
        let project_id = match &self.project_id {
            Some(id) => id,
            None => match &self.project {
                TargetProjectScope::Id(id) => id,
                _ => return Err(TargetError::IdOnly(self.id.clone())),
            },
        };

        // let task_id = match &self.task {
        //     TargetTask::Id(id) => id,
        //     _ => return Err(TargetError::Target(TargetError::IdOnly(self.id.clone()))),
        // };

        Ok((project_id.clone(), self.task_id.clone()))
    }

    pub fn is_all_task(&self, task_id: &str) -> bool {
        if matches!(&self.project, TargetProjectScope::All) {
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
            project: TargetProjectScope::OwnSelf,
            project_id: None,
            task_id: "unknown".into(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format() {
        assert_eq!(Target::format("foo", "build").unwrap(), "foo:build");
    }

    #[test]
    fn format_with_slashes() {
        assert_eq!(
            Target::format("foo/sub", "build/esm").unwrap(),
            "foo/sub:build/esm"
        );
    }

    #[test]
    fn format_node() {
        assert_eq!(
            Target::format("@scope/foo", "build").unwrap(),
            "@scope/foo:build"
        );
    }

    #[test]
    #[should_panic(expected = "InvalidFormat(\"foo$:build\")")]
    fn invalid_chars() {
        Target::parse("foo$:build").unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidFormat(\"foo:@build\")")]
    fn invalid_task_no_at() {
        Target::parse("foo:@build").unwrap();
    }

    #[test]
    fn parse_ids() {
        assert_eq!(
            Target::parse("foo:build").unwrap(),
            Target {
                id: String::from("foo:build"),
                project: TargetProjectScope::Id("foo".to_owned()),
                project_id: Some("foo".to_owned()),
                task_id: "build".to_owned(),
                // task: TargetTask::Id("build".to_owned())
            }
        );
    }

    #[test]
    fn parse_deps_project() {
        assert_eq!(
            Target::parse("^:build").unwrap(),
            Target {
                id: String::from("^:build"),
                project: TargetProjectScope::Deps,
                project_id: None,
                task_id: "build".to_owned(),
                // task: TargetTask::Id("build".to_owned())
            }
        );
    }

    // #[test]
    // fn parse_deps_project_all_tasks() {
    //     assert_eq!(
    //         Target::parse("^:").unwrap(),
    //         Target {
    //             id: String::from("^:"),
    //             project: TargetProjectScope::Deps,
    //             task: TargetTask::All,
    //         }
    //     );
    // }

    #[test]
    fn parse_self_project() {
        assert_eq!(
            Target::parse("~:build").unwrap(),
            Target {
                id: String::from("~:build"),
                project: TargetProjectScope::OwnSelf,
                project_id: None,
                task_id: "build".to_owned(),
                // task: TargetTask::Id("build".to_owned())
            }
        );
    }

    // #[test]
    // fn parse_self_project_all_tasks() {
    //     assert_eq!(
    //         Target::parse("~:").unwrap(),
    //         Target {
    //             id: String::from("~:"),
    //             project: TargetProjectScope::Own,
    //             task: TargetTask::All,
    //         }
    //     );
    // }

    #[test]
    fn parse_all_projects() {
        assert_eq!(
            Target::parse(":build").unwrap(),
            Target {
                id: String::from(":build"),
                project: TargetProjectScope::All,
                project_id: None,
                task_id: "build".to_owned(),
                // task: TargetTask::Id("build".to_owned())
            }
        );
    }

    // #[test]
    // fn parse_all_tasks() {
    //     assert_eq!(
    //         Target::parse("foo:").unwrap(),
    //         Target {
    //             id: String::from("foo:"),
    //             project: TargetProjectScope::Id("foo".to_owned()),
    //             task: TargetTask::All,
    //         }
    //     );
    // }

    #[test]
    #[should_panic(expected = "TooWild")]
    fn parse_too_wild() {
        Target::parse(":").unwrap();
    }

    #[test]
    fn parse_node() {
        assert_eq!(
            Target::parse("@scope/foo:build").unwrap(),
            Target {
                id: String::from("@scope/foo:build"),
                project: TargetProjectScope::Id("@scope/foo".to_owned()),
                project_id: Some("@scope/foo".to_owned()),
                task_id: "build".to_owned(),
                // task: TargetTask::Id("build".to_owned())
            }
        );
    }

    #[test]
    fn parse_slashes() {
        assert_eq!(
            Target::parse("foo/sub:build/esm").unwrap(),
            Target {
                id: String::from("foo/sub:build/esm"),
                project: TargetProjectScope::Id("foo/sub".to_owned()),
                project_id: Some("foo/sub".to_owned()),
                task_id: "build/esm".to_owned(),
                // task: TargetTask::Id("build".to_owned())
            }
        );
    }

    #[test]
    fn matches_all() {
        let all = Target::parse(":lint").unwrap();

        assert!(all.is_all_task("lint"));
        assert!(all.is_all_task(":lint"));
        assert!(!all.is_all_task("build"));
        assert!(!all.is_all_task(":build"));
        assert!(!all.is_all_task("foo:lint"));

        let full = Target::parse("foo:lint").unwrap();

        assert!(!full.is_all_task("lint"));
        assert!(!full.is_all_task(":lint"));
        assert!(!full.is_all_task("build"));
        assert!(!full.is_all_task(":build"));
        assert!(!full.is_all_task("foo:lint"));
    }
}
