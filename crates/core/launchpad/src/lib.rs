use moon_logger::debug;
use moon_utils::semver::Version;
use moon_utils::{get_cache_dir, is_ci, is_offline, is_test_env};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::time::{Duration, SystemTime};

const CURRENT_VERSION_URL: &str = "https://launch.moonrepo.app/versions/cli/current";
const ALERT_PAUSE_DURATION: Duration = Duration::from_secs(3600);

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentVersion {
    pub current_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CheckState {
    pub last_alert: SystemTime,
}

pub async fn check_version(
    local_version_str: &str,
) -> Result<(String, bool), Box<dyn Error + Send + Sync>> {
    if is_test_env() || is_offline() {
        return Ok((env!("CARGO_PKG_VERSION").to_owned(), false));
    }

    debug!("Checking for new version of moon");

    let response = reqwest::Client::new()
        .get(CURRENT_VERSION_URL)
        .header("X-Moon-Version", local_version_str)
        .header("X-Moon-CI", is_ci().to_string())
        .header(
            "X-Moon-ID",
            env::var("MOONBASE_ACCESS_KEY")
                .or_else(|_| env::var("MOONBASE_API_KEY"))
                .unwrap_or_default(),
        )
        .send()
        .await?
        .text()
        .await?;

    let data: CurrentVersion = serde_json::from_str(&response)?;

    let local_version = Version::parse(local_version_str)?;
    let remote_version = Version::parse(data.current_version.as_str())?;

    if remote_version > local_version {
        let check_state_path = get_cache_dir().join("states/versionCheck.json");
        let now = SystemTime::now();

        if let Ok(file) = fs::read_to_string(&check_state_path) {
            let check_state: Result<CheckState, _> = serde_json::from_str(&file);

            if let Ok(state) = check_state {
                if (state.last_alert + ALERT_PAUSE_DURATION) > now {
                    return Ok((remote_version.to_string(), false));
                }
            }
        }

        moon_utils::fs::create_dir_all(check_state_path.parent().unwrap())?;

        let check_state = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&check_state_path)?;

        serde_json::to_writer(check_state, &CheckState { last_alert: now })?;

        return Ok((remote_version.to_string(), true));
    }

    Ok((remote_version.to_string(), false))
}
