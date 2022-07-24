use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub trait Hasher {
    fn hash(&self, sha: &mut Sha256);
}

pub fn to_hash(hashers: &[impl Hasher]) -> String {
    let mut sha = Sha256::new();

    for hasher in hashers {
        hasher.hash(&mut sha);
    }

    format!("{:x}", sha.finalize())
}

pub fn hash_btree(tree: &BTreeMap<String, String>, sha: &mut Sha256) {
    for (k, v) in tree {
        sha.update(k.as_bytes());
        sha.update(v.as_bytes());
    }
}

pub fn hash_vec(list: &Vec<String>, sha: &mut Sha256) {
    for v in list {
        sha.update(v.as_bytes());
    }
}
