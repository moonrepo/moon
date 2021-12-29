use monolith_config::{ProjectID, TargetID};

pub struct Target {}

impl Target {
    pub fn format(project_id: &str, task_name: &str) -> TargetID {
        format!("{}:{}", project_id, task_name)
    }

    pub fn parse(target: &str) -> (ProjectID, String) {
        let split: Vec<&str> = target.split(':').collect();

        (String::from(split[0]), String::from(split[1]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format() {
        assert_eq!(Target::format("foo", "build"), "foo:build");
    }

    #[test]
    fn parse() {
        assert_eq!(
            Target::parse("foo:build"),
            (String::from("foo"), String::from("build"))
        );
    }
}
