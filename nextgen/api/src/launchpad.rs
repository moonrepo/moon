use miette::IntoDiagnostic;
use moon_cache::{cache_item, get_moon_home_dir, CacheEngine};
use moon_common::consts::CONFIG_DIRNAME;
use moon_common::is_test_env;
use semver::Version;
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, json};
use std::env;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tracing::debug;
use uuid::Uuid;

const CURRENT_VERSION_URL: &str = "https://launch.moonrepo.app/versions/cli/current";
const ALERT_PAUSE_DURATION: Duration = Duration::from_secs(43200); // 12 hours

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CurrentVersion {
    pub current_version: String,
    pub message: Option<String>,
}

cache_item!(
    pub struct CurrentVersionState {
        pub last_check: Option<SystemTime>,
        pub local_version: Option<Version>,
        pub remote_version: Option<Version>,
    }
);

fn load_or_create_anonymous_uid() -> miette::Result<String> {
    let id_path = get_moon_home_dir().join("id");

    if id_path.exists() {
        return Ok(fs::read_file(id_path)?);
    }

    let id = Uuid::new_v4().to_string();

    fs::write_file(id_path, &id)?;

    Ok(id)
}

fn create_anonymous_rid(workspace_root: &Path) -> String {
    format!(
        "{:x}",
        md5::compute(
            env::var("MOONBASE_REPO_SLUG")
                .unwrap_or_else(|_| fs::file_name(workspace_root)),
        )
    )
}

pub struct Launchpad;

impl Launchpad {
    pub async fn check_version(
        cache_engine: &CacheEngine,
        current_version: &str,
        bypass_cache: bool,
    ) -> miette::Result<Option<CurrentVersion>> {
        let moon_dir = fs::find_upwards(
            CONFIG_DIRNAME,
            env::current_dir().expect("Invalid working directory."),
        );

        if is_test_env() || proto_core::is_offline() || moon_dir.is_none() {
            return Ok(None);
        }

        // Check cache and avoid requesting if within the pause duration
        let mut state = cache_engine.cache_state::<CurrentVersionState>("moonVersion.json")?;

        if let Some(last_check) = state.data.last_check {
            if (last_check + ALERT_PAUSE_DURATION) > SystemTime::now() && !bypass_cache {
                return Ok(None);
            }
        }

        debug!(current_version, "Checking for new version of moon");

        let Ok(response) = reqwest::Client::new()
            .get(CURRENT_VERSION_URL)
            .header("X-Moon-Version", current_version)
            .header("X-Moon-CI", ci_env::is_ci().to_string())
            .header(
                "X-Moon-CI-Provider",
                format!("{:?}", ci_env::detect_provider()),
            )
            .header("X-Moon-CD", cd_env::is_cd().to_string())
            .header(
                "X-Moon-CD-Provider",
                format!("{:?}", cd_env::detect_provider()),
            )
            .header("X-Moon-UID", load_or_create_anonymous_uid()?)
            .header(
                "X-Moon-RID",
                create_anonymous_rid(moon_dir.unwrap().parent().unwrap()),
            )
            .send()
            .await
        else {
            return Ok(None);
        };

        let Ok(text) = response.text().await else {
            return Ok(None);
        };

        let data: CurrentVersion = json::from_str(&text).into_diagnostic()?;
        let local_version = Version::parse(current_version).into_diagnostic()?;
        let remote_version = Version::parse(&data.current_version).into_diagnostic()?;
        let update_available = remote_version > local_version;

        // Write state to cache
        state.data.last_check = Some(SystemTime::now());
        state.data.local_version = Some(local_version);
        state.data.remote_version = Some(remote_version);
        state.save()?;

        if update_available {
            debug!(
                latest_version = &data.current_version,
                "Found a newer version"
            );

            return Ok(Some(data));
        }

        Ok(None)
    }
}
