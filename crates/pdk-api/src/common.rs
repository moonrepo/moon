use warpgate_api::{api_enum, api_struct, ExecCommandInput, VirtualPath};

api_struct!(
    /// Information about the current moon workspace.
    pub struct MoonContext {
        /// Virtual path to the current working directory.
        pub working_dir: VirtualPath,

        /// Virtual path to the workspace root.
        pub workspace_root: VirtualPath,
    }
);

impl MoonContext {
    /// Return the provided file path as an absolute path (using virtual paths).
    /// If the path is already absolute (either real or virtual), return it.
    /// Otherwise prefix the path with the current working directory.
    pub fn get_absolute_path<T: AsRef<std::path::Path>>(&self, path: T) -> VirtualPath {
        let path = path.as_ref();

        if path.is_absolute() {
            return VirtualPath::OnlyReal(path.to_owned());
        }

        self.working_dir.join(path)
    }
}

api_enum!(
    /// An operation to perform within moon (the host) itself.
    #[serde(tag = "type", rename_all = "kebab-case")]
    pub enum Operation {
        ProcessExecution(ExecCommandInput),
    }
);
