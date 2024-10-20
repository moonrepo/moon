#[derive(Clone, Copy, Default, PartialEq)]
pub enum RepoType {
    #[default]
    Unknown,
    Monorepo,
    MonorepoWithRoot,
    Polyrepo,
}
