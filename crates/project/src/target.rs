use crate::errors::{ProjectError, TargetError};
use moon_config::{ProjectID, TargetID, TaskID};
use moon_utils::regex::TARGET_PATTERN;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum TargetProject {
    All,           // :task
    Deps,          // ^:task
    Id(ProjectID), // id:task
    Own,           // ~:task
}

impl fmt::Display for TargetProject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetProject::All => write!(f, ""),
            TargetProject::Deps => write!(f, "^"),
            TargetProject::Id(name) => write!(f, "{}", name),
            TargetProject::Own => write!(f, "~"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TargetTask {
    All,        // project:
    Id(TaskID), // project:id
}

impl fmt::Display for TargetTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetTask::All => write!(f, ""),
            TargetTask::Id(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Target {
    pub project: TargetProject,
    pub task: TargetTask,
}

impl Target {
    pub fn format(project_id: &str, task_id: &str) -> Result<TargetID, ProjectError> {
        Ok(format!("{}:{}", project_id, task_id))
    }

    pub fn format_with(project: TargetProject, task: TargetTask) -> Result<TargetID, ProjectError> {
        Ok(format!("{}:{}", project, task))
    }

    pub fn parse(target: &str) -> Result<Target, ProjectError> {
        let matches = match TARGET_PATTERN.captures(target) {
            Some(result) => result,
            None => {
                return Err(ProjectError::Target(TargetError::InvalidFormat(
                    String::from(target),
                )))
            }
        };

        let project = match matches.name("project") {
            Some(value) => match value.as_str() {
                "" => TargetProject::All,
                "^" => TargetProject::Deps,
                "~" => TargetProject::Own,
                id => TargetProject::Id(id.to_owned()),
            },
            None => TargetProject::All,
        };

        let task = match matches.name("task") {
            Some(value) => match value.as_str() {
                "" => TargetTask::All,
                id => TargetTask::Id(id.to_owned()),
            },
            None => TargetTask::All,
        };

        Ok(Target { project, task })
    }

    pub fn parse_ids(target: &str) -> Result<(ProjectID, TaskID), ProjectError> {
        let result = Target::parse(target)?;

        let project_id = match result.project {
            TargetProject::Id(id) => id,
            _ => {
                return Err(ProjectError::Target(TargetError::IdOnly(String::from(
                    target,
                ))))
            }
        };

        let task_id = match result.task {
            TargetTask::Id(id) => id,
            _ => {
                return Err(ProjectError::Target(TargetError::IdOnly(String::from(
                    target,
                ))))
            }
        };

        Ok((project_id, task_id))
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
    fn parse_ids() {
        assert_eq!(
            Target::parse("foo:build").unwrap(),
            Target {
                project: TargetProject::Id("foo".to_owned()),
                task: TargetTask::Id("build".to_owned())
            }
        );
    }

    #[test]
    fn parse_deps_project() {
        assert_eq!(
            Target::parse("^:build").unwrap(),
            Target {
                project: TargetProject::Deps,
                task: TargetTask::Id("build".to_owned())
            }
        );
    }

    #[test]
    fn parse_deps_project_all_tasks() {
        assert_eq!(
            Target::parse("^:").unwrap(),
            Target {
                project: TargetProject::Deps,
                task: TargetTask::All,
            }
        );
    }

    #[test]
    fn parse_self_project() {
        assert_eq!(
            Target::parse("~:build").unwrap(),
            Target {
                project: TargetProject::Own,
                task: TargetTask::Id("build".to_owned())
            }
        );
    }

    #[test]
    fn parse_self_project_all_tasks() {
        assert_eq!(
            Target::parse("~:").unwrap(),
            Target {
                project: TargetProject::Own,
                task: TargetTask::All,
            }
        );
    }

    #[test]
    fn parse_all_projects() {
        assert_eq!(
            Target::parse(":build").unwrap(),
            Target {
                project: TargetProject::All,
                task: TargetTask::Id("build".to_owned())
            }
        );
    }

    #[test]
    fn parse_all_tasks() {
        assert_eq!(
            Target::parse("foo:").unwrap(),
            Target {
                project: TargetProject::Id("foo".to_owned()),
                task: TargetTask::All,
            }
        );
    }
}
