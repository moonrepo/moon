#![allow(dead_code)]

pub use bazel_remote_apis::build::bazel::remote::execution::v2::Digest as RemoteDigest;
use moon_hash::{ContentHash, Digest};

pub trait LocalDigestExt {
    fn from_remote(digest: RemoteDigest) -> miette::Result<Digest>;
    fn into_remote_digest(self) -> RemoteDigest;
    fn to_remote_digest(&self) -> RemoteDigest;
}

impl LocalDigestExt for Digest {
    fn from_remote(digest: RemoteDigest) -> miette::Result<Digest> {
        Ok(Digest {
            hash: ContentHash::from_hex(&digest.hash)?,
            size: digest.size_bytes,
        })
    }

    fn into_remote_digest(self) -> RemoteDigest {
        RemoteDigest {
            hash: self.hash.to_string(),
            size_bytes: self.size,
        }
    }

    fn to_remote_digest(&self) -> RemoteDigest {
        RemoteDigest {
            hash: self.hash.to_string(),
            size_bytes: self.size,
        }
    }
}

pub trait RemoteDigestExt {
    fn from_local(digest: Digest) -> RemoteDigest;
    fn into_local_digest(self) -> miette::Result<Digest>;
    fn to_local_digest(&self) -> miette::Result<Digest>;
}

impl RemoteDigestExt for RemoteDigest {
    fn from_local(digest: Digest) -> RemoteDigest {
        RemoteDigest {
            hash: digest.hash.to_string(),
            size_bytes: digest.size,
        }
    }

    fn into_local_digest(self) -> miette::Result<Digest> {
        Ok(Digest {
            hash: ContentHash::from_hex(&self.hash)?,
            size: self.size_bytes,
        })
    }

    fn to_local_digest(&self) -> miette::Result<Digest> {
        Ok(Digest {
            hash: ContentHash::from_hex(&self.hash)?,
            size: self.size_bytes,
        })
    }
}
