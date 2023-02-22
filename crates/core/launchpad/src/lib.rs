use moon_constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_logger::debug;
use moon_utils::semver::Version;
use moon_utils::{fs, get_cache_dir, is_ci, is_test_env, path};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::OpenOptions;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const CURRENT_VERSION_URL: &str = "https://launch.moonrepo.app/versions/cli/current";
const ALERT_PAUSE_DURATION: Duration = Duration::from_secs(28800); // 8 hours

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentVersion {
    pub current_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CheckState {
    pub last_alert: SystemTime,
}

fn load_or_create_anonymous_id() -> Result<String, MoonError> {
    let id_path = path::get_home_dir()
        .unwrap()
        .join(CONFIG_DIRNAME)
        .join("id");

    if id_path.exists() {
        return fs::read(id_path);
    }

    let id = Uuid::new_v4().to_string();

    fs::write(id_path, &id)?;

    return Ok(id);
}

pub async fn check_version(
    local_version_str: &str,
) -> Result<(String, bool), Box<dyn Error + Send + Sync>> {
    if is_test_env() || proto::is_offline() {
        return Ok((local_version_str.to_owned(), false));
    }

    let check_state_path = get_cache_dir().join("states/versionCheck.json");
    let now = SystemTime::now();

    // Only check once every 8 hours
    if let Ok(file) = fs::read(&check_state_path) {
        let check_state: Result<CheckState, _> = serde_json::from_str(&file);

        if let Ok(state) = check_state {
            if (state.last_alert + ALERT_PAUSE_DURATION) > now {
                return Ok((local_version_str.to_owned(), false));
            }
        }
    }

    debug!(target: "moon:launchpad", "Checking for new version of moon");

    let response = reqwest::Client::new()
        .get(CURRENT_VERSION_URL)
        .header("X-Moon-Version", local_version_str)
        .header("X-Moon-CI", is_ci().to_string())
        .header("X-Moon-ID", load_or_create_anonymous_id()?)
        .send()
        .await?
        .text()
        .await?;

    let data: CurrentVersion = serde_json::from_str(&response)?;
    let local_version = Version::parse(local_version_str)?;
    let remote_version = Version::parse(data.current_version.as_str())?;

    if remote_version > local_version {
        fs::create_dir_all(check_state_path.parent().unwrap())?;

        let check_state = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&check_state_path)?;

        serde_json::to_writer(check_state, &CheckState { last_alert: now })?;

        return Ok((remote_version.to_string(), true));
    }

    Ok((remote_version.to_string(), false))
}
