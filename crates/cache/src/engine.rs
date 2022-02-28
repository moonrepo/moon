use crate::items::{CacheItem, RunTargetState, WorkspaceState};
use crate::runfiles::CacheRunfile;
use moon_config::constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_utils::fs;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    /// Contains cached items pertaining to runs and processes.
    pub dir: PathBuf,

    /// The `.moon/out` directory relative to workspace root.
    /// Contains project outputs from configured tasks.
    pub out: PathBuf,
}

impl CacheEngine {
    pub async fn create(workspace_root: &Path) -> Result<Self, MoonError> {
        let dir = workspace_root.join(CONFIG_DIRNAME).join("cache");
        let out = workspace_root.join(CONFIG_DIRNAME).join("out");

        fs::create_dir_all(&dir).await?;
        fs::create_dir_all(&out).await?;

        Ok(CacheEngine { dir, out })
    }

    pub async fn cache_run_target_state(
        &self,
        target: &str,
    ) -> Result<CacheItem<RunTargetState>, MoonError> {
        let path: PathBuf = [&target.replace(':', "/"), "lastRunState.json"]
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

    pub async fn cache_workspace_state(&self) -> Result<CacheItem<WorkspaceState>, MoonError> {
        Ok(CacheItem::load(
            self.dir.join("workspaceState.json"),
            WorkspaceState::default(),
        )
        .await?)
    }

    pub async fn create_runfile<T: DeserializeOwned + Serialize>(
        &self,
        id: &str,
        data: &T,
    ) -> Result<CacheRunfile, MoonError> {
        let path: PathBuf = [id, "runfile.json"].iter().collect();

        Ok(CacheRunfile::load(self.dir.join(path), data).await?)
    }

    pub async fn delete_runfiles(&self) -> Result<(), MoonError> {
        let entries = fs::read_dir(&self.dir).await?;

        for entry in entries {
            let path = entry.path();

            if path.is_dir() {
                fs::remove_file(&path.join("runfile.json")).await?;
            }
        }

        Ok(())
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
            assert!(dir.path().join(".moon/out").exists());

            dir.close().unwrap();
        }
    }

    mod delete_runfiles {
        use super::*;

        #[tokio::test]
        async fn deletes_dir() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();

            let runfile1 = cache
                .create_runfile("123", &"content".to_owned())
                .await
                .unwrap();
            let runfile2 = cache
                .create_runfile("456", &"content".to_owned())
                .await
                .unwrap();

            assert!(runfile1.path.exists());
            assert!(runfile2.path.exists());

            cache.delete_runfiles().await.unwrap();

            assert!(!runfile1.path.exists());
            assert!(!runfile2.path.exists());

            dir.close().unwrap();
        }
    }

    mod create_runfile {
        use super::*;

        #[tokio::test]
        async fn creates_runfile_on_call() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let runfile = cache
                .create_runfile("123", &"content".to_owned())
                .await
                .unwrap();

            assert!(runfile.path.exists());

            assert_eq!(
                fs::read_to_string(dir.path().join(".moon/cache/123/runfile.json")).unwrap(),
                "\"content\""
            );

            dir.close().unwrap();
        }
    }

    mod cache_run_target_state {
        use super::*;

        #[tokio::test]
        async fn creates_parent_dir_on_call() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let item = cache.cache_run_target_state("foo:bar").await.unwrap();

            assert!(!item.path.exists());
            assert!(item.path.parent().unwrap().exists());

            dir.close().unwrap();
        }

        #[tokio::test]
        async fn loads_cache_if_it_exists() {
            let dir = assert_fs::TempDir::new().unwrap();

            dir.child(".moon/cache/foo/bar/lastRunState.json")
                .write_str(r#"{"exitCode":123,"lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let item = cache.cache_run_target_state("foo:bar").await.unwrap();

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
            let mut item = cache.cache_run_target_state("foo:bar").await.unwrap();

            item.item.exit_code = 123;
            item.save().await.unwrap();

            assert_eq!(
                fs::read_to_string(item.path).unwrap(),
                r#"{"exitCode":123,"lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#
            );

            dir.close().unwrap();
        }
    }

    mod cache_workspace_state {
        use super::*;

        #[tokio::test]
        async fn creates_parent_dir_on_call() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let item = cache.cache_workspace_state().await.unwrap();

            assert!(!item.path.exists());
            assert!(item.path.parent().unwrap().exists());

            dir.close().unwrap();
        }

        #[tokio::test]
        async fn loads_cache_if_it_exists() {
            let dir = assert_fs::TempDir::new().unwrap();

            dir.child(".moon/cache/workspaceState.json")
                .write_str(r#"{"lastNodeInstallTime":123}"#)
                .unwrap();

            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let item = cache.cache_workspace_state().await.unwrap();

            assert_eq!(
                item.item,
                WorkspaceState {
                    last_node_install_time: 123
                }
            );

            dir.close().unwrap();
        }

        #[tokio::test]
        async fn saves_to_cache() {
            let dir = assert_fs::TempDir::new().unwrap();
            let cache = CacheEngine::create(dir.path()).await.unwrap();
            let mut item = cache.cache_workspace_state().await.unwrap();

            item.item.last_node_install_time = 123;
            item.save().await.unwrap();

            assert_eq!(
                fs::read_to_string(item.path).unwrap(),
                r#"{"lastNodeInstallTime":123}"#
            );

            dir.close().unwrap();
        }
    }
}
