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
    pub id: String,

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

    pub fn parse(target_id: &str) -> Result<Target, ProjectError> {
        if target_id == ":" {
            return Err(ProjectError::Target(TargetError::TooWild));
        }

        let matches = match TARGET_PATTERN.captures(target_id) {
            Some(result) => result,
            None => {
                return Err(ProjectError::Target(TargetError::InvalidFormat(
                    target_id.to_owned(),
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

        Ok(Target {
            id: target_id.to_owned(),
            project,
            task,
        })
    }

    pub fn block_project_scope(&self, scope: TargetProject) -> Result<(), ProjectError> {
        if matches!(&self.project, scope) {
            return Err(ProjectError::Target(TargetError::UnsupportedScope(
                scope.to_string(),
            )));
        }

        Ok(())
    }

    pub fn block_task_scope(&self, scope: TargetTask) -> Result<(), ProjectError> {
        if matches!(&self.task, scope) {
            return Err(ProjectError::Target(TargetError::UnsupportedScope(
                scope.to_string(),
            )));
        }

        Ok(())
    }

    pub fn ids(&self) -> Result<(ProjectID, TaskID), ProjectError> {
        let project_id = match &self.project {
            TargetProject::Id(id) => id,
            _ => return Err(ProjectError::Target(TargetError::IdOnly(self.id.clone()))),
        };

        let task_id = match &self.task {
            TargetTask::Id(id) => id,
            _ => return Err(ProjectError::Target(TargetError::IdOnly(self.id.clone()))),
        };

        Ok((project_id.clone(), task_id.clone()))
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
                id: String::from("foo:build"),
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
                id: String::from("^:build"),
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
                id: String::from("^:"),
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
                id: String::from("~:build"),
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
                id: String::from("~:"),
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
                id: String::from(":build"),
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
                id: String::from("foo:"),
                project: TargetProject::Id("foo".to_owned()),
                task: TargetTask::All,
            }
        );
    }

    #[test]
    #[should_panic(expected = "Target(TooWild)")]
    fn parse_too_wild() {
        Target::parse(":").unwrap();
    }
}
