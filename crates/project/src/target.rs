use crate::errors::ProjectError;
use moon_config::{ProjectID, TargetID, TaskID};

pub struct Target {}

impl Target {
    pub fn format(project_id: &str, task_id: &str) -> Result<TargetID, ProjectError> {
        Ok(format!("{}:{}", project_id, task_id))
    }

    pub fn parse(target: &str) -> Result<(ProjectID, TaskID), ProjectError> {
        let split: Vec<&str> = target.split(':').collect();

        if split.len() != 2 {
            return Err(ProjectError::InvalidTargetFormat(String::from(target)));
        }

        Ok((String::from(split[0]), String::from(split[1])))
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
    fn parse() {
        assert_eq!(
            Target::parse("foo:build").unwrap(),
            (String::from("foo"), String::from("build"))
        );
    }
}
