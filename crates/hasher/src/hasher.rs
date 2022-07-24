use sha2::{Digest, Sha256};

pub trait Hasher {
    /// Convert the hasher and its contents to a SHA256 hash.
    fn to_hash(&self) -> String;
}

pub fn create_sha256() -> Sha256 {
    Sha256::new()
}
