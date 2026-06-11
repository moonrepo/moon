use crate::digest_compat::{LocalDigestExt, RemoteDigestExt};
use crate::manifest_files::{ManifestFile, ManifestSymlink};
use bazel_remote_apis::build::bazel::remote::execution::v2::{Action, ActionResult};
use moon_hash::Digest;

#[derive(Debug, Default)]
pub struct Manifest {
    pub digest: Option<Digest>,
}

impl Manifest {
    pub fn from_bazel_action(action: Action) -> miette::Result<Self> {
        Ok(Self {
            digest: match action.command_digest {
                Some(digest) => Some(digest.to_local_digest()?),
                None => None,
            },
        })
    }

    pub fn into_bazel_action(self) -> Action {
        Action {
            command_digest: self.digest.as_ref().map(|digest| digest.to_remote_digest()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct ManifestResult {
    pub files: Vec<ManifestFile>,
    pub symlinks: Vec<ManifestSymlink>,
    pub exit_code: i32,
    pub stderr_digest: Option<Digest>,
    pub stdout_digest: Option<Digest>,
}

impl ManifestResult {
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
                Some(digest) => Some(digest.to_local_digest()?),
                None => None,
            },
            stdout_digest: match result.stdout_digest {
                Some(digest) => Some(digest.to_local_digest()?),
                None => None,
            },
            ..Default::default()
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
            stderr_digest: self.stderr_digest.map(|digest| digest.to_remote_digest()),
            stdout_digest: self.stdout_digest.map(|digest| digest.to_remote_digest()),
            ..Default::default()
        }
    }
}
