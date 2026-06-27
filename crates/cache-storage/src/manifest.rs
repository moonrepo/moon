use crate::digest_compat::{ExternalDigestExt, InternalDigestExt};
use crate::helpers::{create_from_timestamp, create_timestamp};
use crate::storage_backend::BoxedStorageBackend;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, ExecutedActionMetadata, NodeProperties, OutputFile, OutputSymlink,
};
use bazel_remote_apis::google::protobuf::UInt32Value;
use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_hash::Digest;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct ManifestSource {
    pub backend: BoxedStorageBackend,
    pub manifest: Manifest,
    pub remote: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Manifest {
    // Outputs
    pub files: Vec<ManifestFile>,
    pub symlinks: Vec<ManifestSymlink>,

    // Process
    pub exit_code: i32,
    #[serde(skip)]
    pub stderr_bytes: Option<Bytes>,
    pub stderr_digest: Option<Digest>,
    #[serde(skip)]
    pub stdout_bytes: Option<Bytes>,
    pub stdout_digest: Option<Digest>,

    // Timings
    pub upload_started_at: Option<SystemTime>,
    pub upload_completed_at: Option<SystemTime>,
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
            stderr_bytes: if result.stderr_digest.is_some() {
                Some(Bytes::from(result.stderr_raw))
            } else {
                None
            },
            stderr_digest: match result.stderr_digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            },
            stdout_bytes: if result.stdout_digest.is_some() {
                Some(Bytes::from(result.stdout_raw))
            } else {
                None
            },
            stdout_digest: match result.stdout_digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            },
            upload_completed_at: result
                .execution_metadata
                .take()
                .and_then(|metadata| metadata.output_upload_completed_timestamp)
                .map(create_from_timestamp),
            upload_started_at: result
                .execution_metadata
                .take()
                .and_then(|metadata| metadata.output_upload_start_timestamp)
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
            stderr_raw: self
                .stderr_bytes
                .map(|bytes| bytes.to_vec())
                .unwrap_or_default(),
            stdout_digest: self.stdout_digest.map(|digest| digest.to_external_digest()),
            stdout_raw: self
                .stdout_bytes
                .map(|bytes| bytes.to_vec())
                .unwrap_or_default(),
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

    pub fn hydrate(&mut self, blobs: &FxHashMap<Digest, BlobContent>) {
        if self.stderr_bytes.is_none()
            && let Some(digest) = &self.stderr_digest
            && let Some(content) = blobs.get(digest)
            && let Some(bytes) = content.get_bytes()
        {
            self.stderr_bytes = Some(Bytes::from(bytes.to_vec()));
        }

        if self.stdout_bytes.is_none()
            && let Some(digest) = &self.stdout_digest
            && let Some(content) = blobs.get(digest)
            && let Some(bytes) = content.get_bytes()
        {
            self.stdout_bytes = Some(Bytes::from(bytes.to_vec()));
        }

        for file in &mut self.files {
            if file.bytes.is_some() || file.source_path.is_some() {
                continue;
            }

            if let Some(digest) = &file.digest
                && let Some(content) = blobs.get(digest)
            {
                match content {
                    BlobContent::File(path) => {
                        file.source_path = Some(path.clone());
                    }
                    BlobContent::Inline(bytes) => {
                        file.bytes = Some(Bytes::from(bytes.to_vec()));
                    }
                };
            }
        }
    }

    pub fn is_hydrated(&self) -> bool {
        // A blob is resolved when its bytes are present, when there's no digest,
        // or when the digest is for empty content (size 0) — empty outputs carry
        // no blob and are reconstructed directly during hydration.
        let resolved = |bytes: &Option<Bytes>, digest: &Option<Digest>| {
            bytes.is_some() || digest.as_ref().is_none_or(|digest| digest.size == 0)
        };

        resolved(&self.stderr_bytes, &self.stderr_digest)
            && resolved(&self.stdout_bytes, &self.stdout_digest)
            && self
                .files
                .iter()
                .all(|file| file.source_path.is_some() || resolved(&file.bytes, &file.digest))
    }

    pub fn collect_unhydrated_blob_digests(&self) -> Vec<Digest> {
        let mut digests = vec![];

        if self.stderr_bytes.is_none()
            && let Some(digest) = &self.stderr_digest
            && digest.size > 0
        {
            digests.push(digest.to_owned());
        }

        if self.stdout_bytes.is_none()
            && let Some(digest) = &self.stdout_digest
            && digest.size > 0
        {
            digests.push(digest.to_owned());
        }

        for file in &self.files {
            if file.bytes.is_none()
                && let Some(digest) = &file.digest
                && digest.size > 0
            {
                digests.push(digest.to_owned());
            }
        }

        digests
    }

    pub fn collect_blob_inputs(&self, workspace_root: &Path) -> Vec<BlobInput> {
        let mut sources = vec![];

        if let (Some(digest), Some(bytes)) = (&self.stderr_digest, &self.stderr_bytes) {
            sources.push(BlobInput {
                content: BlobContent::Inline(bytes.clone()),
                digest: digest.to_owned(),
            });
        }

        if let (Some(digest), Some(bytes)) = (&self.stdout_digest, &self.stdout_bytes) {
            sources.push(BlobInput {
                content: BlobContent::Inline(bytes.clone()),
                digest: digest.to_owned(),
            });
        }

        for file in &self.files {
            if let Some(digest) = &file.digest {
                if let Some(bytes) = &file.bytes {
                    sources.push(BlobInput {
                        content: BlobContent::Inline(bytes.clone()),
                        digest: digest.to_owned(),
                    });
                } else {
                    sources.push(BlobInput {
                        content: BlobContent::File(file.path.to_logical_path(workspace_root)),
                        digest: digest.to_owned(),
                    });
                }
            }
        }

        sources
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ManifestFile {
    #[serde(skip)]
    pub bytes: Option<Bytes>,
    pub digest: Option<Digest>,
    pub is_executable: bool,
    pub modified_at: Option<SystemTime>,
    pub path: WorkspaceRelativePathBuf,
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
    pub unix_mode: Option<u32>,
}

impl ManifestFile {
    pub fn from_bazel_file(file: OutputFile) -> miette::Result<Self> {
        let props = file.node_properties.unwrap_or_default();

        Ok(Self {
            // Only retain inline bytes the action result actually inlined. An
            // empty `contents` with a digest means the blob lives on disk and
            // must be sourced from its path (see `collect_blob_sources`), not
            // stored as an empty inline blob under the real (non-empty) digest.
            bytes: if file.digest.is_some() && !file.contents.is_empty() {
                Some(Bytes::from(file.contents))
            } else {
                None
            },
            digest: match file.digest {
                Some(digest) => Some(digest.to_internal_digest()?),
                None => None,
            },
            is_executable: file.is_executable,
            modified_at: props.mtime.map(create_from_timestamp),
            path: file.path.into(),
            source_path: None,
            unix_mode: props.unix_mode.map(|mode| mode.value),
        })
    }

    pub fn into_bazel_file(self) -> OutputFile {
        OutputFile {
            contents: self.bytes.map(|bytes| bytes.to_vec()).unwrap_or_default(),
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
