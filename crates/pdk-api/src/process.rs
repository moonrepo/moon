use crate::{Id, VirtualPath};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use std::collections::BTreeMap;
use warpgate_api::{api_enum, api_struct};

api_struct!(
    /// A native executable requested by an opt-in plugin.
    pub struct ProcessCapabilityDeclaration {
        /// Plugin-local identifier used by subsequent process requests.
        pub id: Id,
        /// Executable name to resolve through the host's PATH.
        pub executable: String,
    }
);

api_struct!(
    /// An opaque process command requested by an opt-in plugin.
    pub struct ProcessCommandInput {
        /// Process capability declared during plugin registration.
        pub capability: Id,
        #[serde(default)]
        pub args: Vec<String>,
        /// Optional working directory. The host confines this to the workspace.
        pub cwd: Option<VirtualPath>,
        /// Environment variables to set or override for the child process.
        #[serde(default)]
        pub env: BTreeMap<String, String>,
    }
);

api_struct!(
    /// Metadata for completed process output stored by the host.
    pub struct ProcessCommandResult {
        pub result_id: u64,
        pub exit_code: i32,
        pub stderr_len: u64,
        pub stdout_len: u64,
    }
);

api_enum!(
    #[derive(Default)]
    #[serde(rename_all = "kebab-case")]
    pub enum ProcessOutputStream {
        Stderr,
        #[default]
        Stdout,
    }
);

api_struct!(
    /// Request a fixed-size chunk of completed process output.
    pub struct ProcessOutputChunkInput {
        pub result_id: u64,
        pub stream: ProcessOutputStream,
        pub offset: u64,
    }
);

api_struct!(
    /// Byte-safe process output chunk returned by the host.
    pub struct ProcessOutputChunk {
        /// Base64-encoded output bytes.
        pub bytes_base64: String,
        pub eof: bool,
    }
);

impl ProcessOutputChunk {
    pub fn from_bytes(bytes: &[u8], eof: bool) -> Self {
        Self {
            bytes_base64: BASE64.encode(bytes),
            eof,
        }
    }

    pub fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        BASE64.decode(&self.bytes_base64)
    }
}

api_struct!(
    pub struct ProcessOutputCloseInput {
        pub result_id: u64,
    }
);

api_struct!(
    pub struct ProcessOutputCloseOutput {}
);

#[derive(Debug)]
pub struct ProcessCommandOutput {
    pub exit_code: i32,
    pub stderr: Vec<u8>,
    pub stdout: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_process_command_contract() {
        let input = ProcessCommandInput {
            capability: Id::raw("vcs"),
            args: vec!["status".into(), "--json".into()],
            cwd: Some(VirtualPath::new("/workspace/project")),
            env: BTreeMap::from([("NO_COLOR".into(), "1".into())]),
        };

        assert_eq!(
            serde_json::to_value(input).unwrap(),
            serde_json::json!({
                "capability": "vcs",
                "args": ["status", "--json"],
                "cwd": "/workspace/project",
                "env": {"NO_COLOR": "1"},
            })
        );
    }

    #[test]
    fn preserves_process_output_chunk_bytes() {
        let chunk = ProcessOutputChunk::from_bytes(&[0xff, 0, b'a'], true);
        let json = serde_json::to_string(&chunk).unwrap();
        let chunk: ProcessOutputChunk = serde_json::from_str(&json).unwrap();

        assert_eq!(chunk.decode().unwrap(), [0xff, 0, b'a']);
        assert!(chunk.eof);
    }
}
