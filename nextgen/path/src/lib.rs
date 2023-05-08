pub enum PathType {
    File(String),
    Glob(String),
}

pub enum Location<T> {
    Absolute(T),
    WorkspaceRelative(T),
    ProjectRelative(T),
}
