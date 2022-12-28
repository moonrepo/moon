use sha2::Sha256;

pub trait Hasher: erased_serde::Serialize {
    fn hash(&self, sha: &mut Sha256);
}
