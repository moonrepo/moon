use crate::{hash_btree, hash_vec, Digest, Hasher, Sha256};
use moon_task::Task;
use moon_utils::path;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetHasher {
    // Task `command`
    command: String,

    // Task `args`
    args: Vec<String>,

    // Task `deps`
    deps: Vec<String>,

    // Environment variables
    env_vars: BTreeMap<String, String>,

    // Input files and globs mapped to a unique hash
    inputs: BTreeMap<String, String>,

    // `project.yml` `dependsOn`
    project_deps: Vec<String>,

    // Task `target`
    target: String,

    // Version of our hasher
    #[allow(dead_code)]
    version: String,
}

impl TargetHasher {
    pub fn new() -> Self {
        TargetHasher {
            version: String::from("1"),
            ..TargetHasher::default()
        }
    }

    /// Hash additional args outside of the provided task.
    pub fn hash_args(&mut self, passthrough_args: &[String]) {
        if !passthrough_args.is_empty() {
            for arg in passthrough_args {
                self.args.push(arg.clone());
            }

            // Sort vectors to be deterministic
            self.args.sort();
        }
    }

    /// Hash a mapping of input file paths to unique file hashes.
    /// File paths *must* be relative from the workspace root.
    pub fn hash_inputs(&mut self, inputs: BTreeMap<String, String>) {
        for (file, hash) in inputs {
            // Standardize on `/` separators so that the hash is
            // the same between windows and nix machines.
            self.inputs.insert(path::standardize_separators(file), hash);
        }
    }

    /// Hash `dependsOn` from the owning project.
    pub fn hash_project_deps(&mut self, deps: Vec<String>) {
        self.project_deps = deps; // Sorted
    }

    /// Hash `args`, `inputs`, `deps`, and `env` vars from a task.
    pub fn hash_task(&mut self, task: &Task) {
        self.command = task.command.clone();
        self.args = task.args.clone();
        self.deps = task.deps.clone();
        self.target = task.target.clone();

        // Sort vectors to be deterministic
        self.args.sort();
        self.deps.sort();
    }
}

impl Hasher for TargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        // Order is important! Do not move things around as it will
        // change the hash and break deterministic builds!
        sha.update(self.version.as_bytes());
        sha.update(self.command.as_bytes());

        hash_vec(&self.args, sha);
        hash_vec(&self.deps, sha);
        hash_btree(&self.env_vars, sha);
        hash_btree(&self.inputs, sha);
        hash_vec(&self.project_deps, sha);
    }
}
