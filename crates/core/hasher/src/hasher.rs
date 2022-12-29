use sha2::Sha256;

pub trait Hasher: Send {
    fn hash(&self, sha: &mut Sha256);

    // This method purely exists because we can't extend Serialize for trait objects!
    fn serialize(&self) -> serde_json::Value;
}
