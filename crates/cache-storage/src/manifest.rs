use crate::digest_compat::{ExternalDigestExt, InternalDigestExt};
use crate::helpers::{create_from_timestamp, create_timestamp};
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, NodeProperties, OutputFile, OutputSymlink,
};
use bazel_remote_apis::google::protobuf::UInt32Value;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_hash::Digest;
use std::time::SystemTime;

#[derive(Debug, Default)]
pub struct Manifest {
    pub files: Vec<ManifestFile>,
    pub symlinks: Vec<ManifestSymlink>,
    pub exit_code: i32,
    pub stderr_digest: Option<Digest>,
    pub stdout_digest: Option<Digest>,
}

impl Manifest {
    pub fn from_bazel_action_result(result: ActionResult) -> miette::Result<Self> {
        let mut files = vec![];
        let mut symlinks = vec![];

        for file in result.output_files {
            files.push(ManifestFile::from_bazel_file(file)?);
        }

        for symlink in result.output_symlinks {
            symlinks.push(ManifestSymlink::from_bazel_symlink(symlink)?);
        }

        Ok(Self {
            files,
            symlinks,
            exit_code: result.exit_code,
            stderr_digest: match result.stderr_digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            },
            stdout_digest: match result.stdout_digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            }
        })
    }

    pub fn into_bazel_action_result(self) -> ActionResult {
        ActionResult {
            output_files: self
                .files
                .into_iter()
                .map(|file| file.into_bazel_file())
                .collect(),
            output_symlinks: self
                .symlinks
                .into_iter()
                .map(|symlink| symlink.into_bazel_symlink())
                .collect(),
            exit_code: self.exit_code,
            stderr_digest: self.stderr_digest.map(|digest| digest.to_external_digest()),
            stdout_digest: self.stdout_digest.map(|digest| digest.to_external_digest()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct ManifestFile {
    pub bytes: Vec<u8>,
    pub digest: Option<Digest>,
    pub is_executable: bool,
    pub modified_at: Option<SystemTime>,
    pub path: WorkspaceRelativePathBuf,
    pub unix_mode: Option<u32>,
}

impl ManifestFile {
    pub fn from_bazel_file(file: OutputFile) -> miette::Result<Self> {
        let props = file.node_properties.unwrap_or_default();

        Ok(Self {
            bytes: file.contents,
            digest: match file.digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            },
            is_executable: file.is_executable,
            modified_at: props.mtime.map(create_from_timestamp),
            path: file.path.into(),
            unix_mode: props.unix_mode.map(|mode| mode.value),
        })
    }

    pub fn into_bazel_file(self) -> OutputFile {
        OutputFile {
            contents: self.bytes,
            digest: self.digest.map(|digest| digest.to_external_digest()),
            is_executable: self.is_executable,
            path: self.path.to_string(),
            node_properties: Some(NodeProperties {
                mtime: self.modified_at.and_then(create_timestamp),
                unix_mode: self.unix_mode.map(|mode| UInt32Value { value: mode }),
                ..Default::default()
            }),
        }
    }
}

#[derive(Debug, Default)]
pub struct ManifestSymlink {
    pub modified_at: Option<SystemTime>,
    pub path: WorkspaceRelativePathBuf,
    pub target: WorkspaceRelativePathBuf,
    pub unix_mode: Option<u32>,
}

impl ManifestSymlink {
    pub fn from_bazel_symlink(file: OutputSymlink) -> miette::Result<Self> {
        let props = file.node_properties.unwrap_or_default();

        Ok(Self {
            modified_at: props.mtime.map(create_from_timestamp),
            path: file.path.into(),
            target: file.target.into(),
            unix_mode: props.unix_mode.map(|mode| mode.value),
        })
    }

    pub fn into_bazel_symlink(self) -> OutputSymlink {
        OutputSymlink {
            path: self.path.to_string(),
            target: self.target.to_string(),
            node_properties: Some(NodeProperties {
                mtime: self.modified_at.and_then(create_timestamp),
                unix_mode: self.unix_mode.map(|mode| UInt32Value { value: mode }),
                ..Default::default()
            }),
        }
    }
}
