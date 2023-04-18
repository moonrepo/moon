use moon_config::ProjectType;

pub enum Operator {
    Equal,    // =
    NotEqual, // !=
    Like,     // ~
    NotLike,  // !~
}

#[derive(Debug, PartialEq)]
pub struct ProjectQuery {
    pub type_of: ProjectType,
}
