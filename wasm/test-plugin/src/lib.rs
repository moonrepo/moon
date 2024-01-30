use moon_common::Id;
use moon_config::WorkspaceConfig;
use moon_target::Target;

pub fn test() {
    dbg!(
        Target::parse("~:target").unwrap(),
        Id::new("id").unwrap(),
        WorkspaceConfig::default()
    );
}
