#![allow(dead_code)]

pub use bazel_remote_apis::build::bazel::remote::execution::v2::Digest as ExternalDigest;
use moon_hash::{ContentHash, Digest};

pub trait InternalDigestExt {
    fn from_external(digest: ExternalDigest) -> miette::Result<Digest>;
    fn into_external_digest(self) -> ExternalDigest;
    fn to_external_digest(&self) -> ExternalDigest;
}

impl InternalDigestExt for Digest {
    fn from_external(digest: ExternalDigest) -> miette::Result<Digest> {
        Ok(Digest {
            hash: ContentHash::from_hex(&digest.hash)?,
            size: digest.size_bytes,
        })
    }

    fn into_external_digest(self) -> ExternalDigest {
        self.to_external_digest()
    }

    fn to_external_digest(&self) -> ExternalDigest {
        ExternalDigest {
            hash: self.hash.to_string(),
            size_bytes: self.size,
        }
    }
}

pub trait ExternalDigestExt {
    fn from_internal(digest: Digest) -> ExternalDigest;
    fn into_internal_digest(self) -> miette::Result<Digest>;
    fn to_internal_digest(&self) -> miette::Result<Digest>;
}

impl ExternalDigestExt for ExternalDigest {
    fn from_internal(digest: Digest) -> ExternalDigest {
        ExternalDigest {
            hash: digest.hash.to_string(),
            size_bytes: digest.size,
        }
    }

    fn into_internal_digest(self) -> miette::Result<Digest> {
        self.to_internal_digest()
    }

    fn to_internal_digest(&self) -> miette::Result<Digest> {
        Ok(Digest {
            hash: ContentHash::from_hex(&self.hash)?,
            size: self.size_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_internal_external_internal() {
        let digest = Digest::from_bytes(b"round trip").unwrap();

        let restored = digest.to_external_digest().to_internal_digest().unwrap();

        assert_eq!(digest, restored);
    }

    #[test]
    fn preserves_hash_and_size() {
        let digest = Digest::from_bytes(b"payload").unwrap();

        let external = digest.to_external_digest();

        assert_eq!(external.hash, digest.hash.to_string());
        assert_eq!(external.size_bytes, digest.size);
    }

    #[test]
    fn rejects_invalid_hex_hash() {
        let external = ExternalDigest {
            hash: "not-a-valid-hash".into(),
            size_bytes: 0,
        };

        assert!(external.to_internal_digest().is_err());
        assert!(Digest::from_external(external).is_err());
    }
}
