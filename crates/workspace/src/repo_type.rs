#[derive(Clone, Copy, Default, PartialEq)]
pub enum RepoType {
    #[default]
    Unknown,
    Monorepo,
    MonorepoWithRoot,
    Polyrepo,
}

impl RepoType {
    pub fn is_monorepo(&self) -> bool {
        matches!(self, Self::Monorepo | Self::MonorepoWithRoot)
    }
}
