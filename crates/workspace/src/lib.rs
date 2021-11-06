mod config;

use config::workspace::WorkspaceConfig;
use std::path::Path;

pub struct Workspace {
    config: WorkspaceConfig,

    root_path: Path,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
