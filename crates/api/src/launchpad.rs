use miette::IntoDiagnostic;
use moon_cache::{CacheEngine, cache_item};
use moon_common::{consts::CONFIG_DIRNAME, is_test_env};
use moon_env::MoonEnvironment;
use moon_env_var::GlobalEnvBag;
use moon_time::now_millis;
use semver::Version;
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, json};
use std::env::{self, consts};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, instrument};
use uuid::Uuid;

const ALERT_PAUSE_DURATION: Duration = Duration::from_secs(43200); // 12 hours

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CurrentVersion {
    pub current_version: String,
    pub message: Option<String>,
}

cache_item!(
    pub struct CurrentVersionCacheState {
        pub last_check_time: Option<u128>,
        pub local_version: Option<Version>,
        pub remote_version: Option<Version>,
    }
);

fn load_or_create_anonymous_uid(id_path: &Path) -> miette::Result<String> {
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
            GlobalEnvBag::instance()
                .get("MOON_VCS_REPO_SLUG")
                .unwrap_or_else(|| fs::file_name(workspace_root)),
        )
    )
}

pub struct VersionCheck {
    pub local_version: Version,
    pub remote_version: Version,
    pub message: Option<String>,
    pub update_available: bool,
}

pub struct Launchpad;

impl Launchpad {
    #[instrument(skip_all)]
    pub async fn check_version(
        cache_engine: &CacheEngine,
        moon_env: &MoonEnvironment,
        bypass_cache: bool,
        manifest_url: &str,
    ) -> miette::Result<Option<VersionCheck>> {
        let mut state = cache_engine
            .state
            .load_state::<CurrentVersionCacheState>("moonVersionCheck.json")?;
        let now = now_millis();

        if let Some(last_check) = state.data.last_check_time {
            if (last_check + ALERT_PAUSE_DURATION.as_millis()) > now && !bypass_cache {
                return Ok(None);
            }
        }

        if let Some(result) = Self::check_version_without_cache(moon_env, manifest_url).await? {
            state.data.last_check_time = Some(now);
            state.data.local_version = Some(result.local_version.clone());
            state.data.remote_version = Some(result.remote_version.clone());
            state.save()?;

            return Ok(Some(result));
        }

        Ok(None)
    }

    pub async fn check_version_without_cache(
        moon_env: &MoonEnvironment,
        manifest_url: &str,
    ) -> miette::Result<Option<VersionCheck>> {
        if is_test_env() || proto_core::is_offline() {
            return Ok(None);
        }

        let version = GlobalEnvBag::instance()
            .get("MOON_VERSION")
            .unwrap_or_default();

        debug!(
            current_version = &version,
            manifest_url = manifest_url,
            "Checking for a new version of moon"
        );

        let mut client = reqwest::Client::new()
            .get(manifest_url)
            .header("X-Moon-OS", consts::OS.to_owned())
            .header("X-Moon-Arch", consts::ARCH.to_owned())
            .header("X-Moon-Version", &version)
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
            .header(
                "X-Moon-UID",
                load_or_create_anonymous_uid(&moon_env.id_file)?,
            );

        if let Some(moon_dir) = fs::find_upwards(
            CONFIG_DIRNAME,
            env::current_dir().expect("Invalid working directory!"),
        ) {
            client = client.header(
                "X-Moon-RID",
                create_anonymous_rid(moon_dir.parent().unwrap()),
            );
        }

        let Ok(response) = client.send().await else {
            return Ok(None);
        };

        let Ok(text) = response.text().await else {
            return Ok(None);
        };

        let data: CurrentVersion = json::parse(text)?;
        let local_version = Version::parse(&version).into_diagnostic()?;
        let remote_version = Version::parse(&data.current_version).into_diagnostic()?;
        let update_available = remote_version > local_version;

        if update_available {
            debug!(
                latest_version = &data.current_version,
                "Found a newer version"
            );
        }

        Ok(Some(VersionCheck {
            local_version,
            remote_version,
            message: data.message,
            update_available,
        }))
    }
}
