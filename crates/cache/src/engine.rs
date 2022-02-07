use crate::items::{CacheItem, RunTargetState, WorkspaceState};
use crate::runfiles::CacheRunfile;
use moon_error::MoonError;
use moon_utils::fs;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    pub dir: PathBuf,
}

impl CacheEngine {
    pub async fn create(workspace_root: &Path) -> Result<Self, MoonError> {
        let dir = workspace_root.join(".moon/cache");

        fs::create_dir_all(&dir).await?;

        Ok(CacheEngine { dir })
    }

    pub async fn delete_runfiles(&self) -> Result<(), MoonError> {
        fs::remove_dir_all(&self.dir.join("runfiles")).await?;

        Ok(())
    }

    pub async fn runfile<T: DeserializeOwned + Serialize>(
        &self,
        path: &str,
        id: &str,
        data: &T,
    ) -> Result<CacheRunfile, MoonError> {
        let path: PathBuf = ["runfiles", path, &format!("{}.json", id)].iter().collect();

        Ok(CacheRunfile::load(self.dir.join(path), data).await?)
    }

    pub async fn run_target_state(
        &self,
        target: &str,
    ) -> Result<CacheItem<RunTargetState>, MoonError> {
        let path: PathBuf = ["runs", &target.replace(':', "/"), "lastState.json"]
            .iter()
            .collect();

        Ok(CacheItem::load(
            self.dir.join(path),
            RunTargetState {
                target: String::from(target),
                ..RunTargetState::default()
            },
        )
        .await?)
    }

    pub async fn workspace_state(&self) -> Result<CacheItem<WorkspaceState>, MoonError> {
        Ok(CacheItem::load(
            self.dir.join("workspaceState.json"),
            WorkspaceState::default(),
        )
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::fs;

    mod create {
        use super::*;

        #[tokio::test]
        async fn creates_dir() {
            let dir = assert_fs::TempDir::new().unwrap();

            CacheEngine::create(dir.path()).await.unwrap();

            assert!(dir.path().join(".moon/cache").exists());

            dir.close().unwrap();
        }
    }

    mod delete_runfiles {
        use super::*;

        #[tokio::test]
        async fn deletes_dir() {
            let dir = assert_fs::TempDir::new().unwrap();

            dir.child(".moon/cache/runfiles").create_dir_all().unwrap();

            let cache = CacheEngine::create(dir.path()).await.unwrap();

            assert!(dir.path().join(".moon/cache/runfiles").exists());

            cache.delete_runfiles().await.unwrap();

            assert!(!dir.path().join(".moon/cache/runfiles").exists());

            dir.close().unwrap();
        }
    }

    mod runfile {
        use super::*;

        #[tokio::test]
        async fn creates_runfile_on_call() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let runfile = cache
                .runfile("tests", "123", &"content".to_owned())
                .await
                .unwrap();

            assert!(runfile.path.exists());

            assert_eq!(
                fs::read_to_string(dir.path().join(".moon/cache/runfiles/tests/123.json")).unwrap(),
                "\"content\""
            );

            dir.close().unwrap();
        }
    }

    mod run_target_state {
        use super::*;

        #[tokio::test]
        async fn creates_parent_dir_on_call() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let item = cache.run_target_state("foo:bar").await.unwrap();

            assert!(!item.path.exists());
            assert!(item.path.parent().unwrap().exists());

            dir.close().unwrap();
        }

        #[tokio::test]
        async fn loads_cache_if_it_exists() {
            let dir = assert_fs::TempDir::new().unwrap();

            dir.child(".moon/cache/runs/foo/bar/lastState.json")
                .write_str(r#"{"exitCode":123,"lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let item = cache.run_target_state("foo:bar").await.unwrap();

            assert_eq!(
                item.item,
                RunTargetState {
                    exit_code: 123,
                    target: String::from("foo:bar"),
                    ..RunTargetState::default()
                }
            );

            dir.close().unwrap();
        }

        #[tokio::test]
        async fn saves_to_cache() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let mut item = cache.run_target_state("foo:bar").await.unwrap();

            item.item.exit_code = 123;
            item.save().await.unwrap();

            assert_eq!(
                fs::read_to_string(item.path).unwrap(),
                r#"{"exitCode":123,"lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#
            );

            dir.close().unwrap();
        }
    }
}
