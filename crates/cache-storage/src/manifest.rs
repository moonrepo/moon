use crate::digest_compat::{ExternalDigestExt, InternalDigestExt};
use crate::helpers::{create_from_timestamp, create_timestamp};
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, ExecutedActionMetadata, NodeProperties, OutputFile, OutputSymlink,
};
use bazel_remote_apis::google::protobuf::UInt32Value;
use moon_blob::{BlobContent, BlobSource, Bytes};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_hash::Digest;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub enum ManifestSource {
    Local(Manifest),
    // LocalShared(Manifest),
    Remote(Manifest),
}

impl ManifestSource {
    pub fn as_manifest(&self) -> &Manifest {
        match self {
            Self::Local(inner) | Self::Remote(inner) => inner,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Manifest {
    // Outputs
    pub files: Vec<ManifestFile>,
    pub symlinks: Vec<ManifestSymlink>,

    // Process
    pub exit_code: i32,
    pub stderr_bytes: Vec<u8>,
    pub stderr_digest: Option<Digest>,
    pub stdout_bytes: Vec<u8>,
    pub stdout_digest: Option<Digest>,

    // Timings
    pub upload_completed_at: Option<SystemTime>,
    pub upload_started_at: Option<SystemTime>,
}

impl Manifest {
    pub fn from_bazel_action_result(mut result: ActionResult) -> miette::Result<Self> {
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
            stderr_bytes: result.stderr_raw,
            stderr_digest: match result.stderr_digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            },
            stdout_bytes: result.stdout_raw,
            stdout_digest: match result.stdout_digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            },
            upload_started_at: result
                .execution_metadata
                .take()
                .and_then(|metadata| metadata.output_upload_start_timestamp)
                .map(create_from_timestamp),
            upload_completed_at: result
                .execution_metadata
                .take()
                .and_then(|metadata| metadata.output_upload_completed_timestamp)
                .map(create_from_timestamp),
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
            execution_metadata: Some(ExecutedActionMetadata {
                worker: "moon".into(),
                output_upload_completed_timestamp: self
                    .upload_completed_at
                    .and_then(create_timestamp),
                output_upload_start_timestamp: self.upload_started_at.and_then(create_timestamp),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn collect_blob_sources(&self) -> Vec<BlobSource> {
        let mut sources = vec![];

        if let (Some(digest), bytes) = (&self.stderr_digest, &self.stderr_bytes) {
            sources.push(BlobSource {
                content: BlobContent::Inline(Bytes::from(bytes.to_vec())),
                digest: digest.to_owned(),
            });
        }

        if let (Some(digest), bytes) = (&self.stdout_digest, &self.stdout_bytes) {
            sources.push(BlobSource {
                content: BlobContent::Inline(Bytes::from(bytes.to_vec())),
                digest: digest.to_owned(),
            });
        }

        for file in &self.files {
            if let Some(digest) = &file.digest {
                sources.push(BlobSource {
                    content: BlobContent::File(file.path.clone()),
                    digest: digest.to_owned(),
                });
            }
        }

        sources
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
