use moon_constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_logger::debug;
use moon_utils::semver::Version;
use moon_utils::{fs, is_ci, is_test_env, path};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::path::Path;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const CURRENT_VERSION_URL: &str = "https://launch.moonrepo.app/versions/cli/current";
const ALERT_PAUSE_DURATION: Duration = Duration::from_secs(43200); // 12 hours

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentVersion {
    pub current_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CheckState {
    pub last_alert: SystemTime,
}

fn load_or_create_anonymous_uid() -> Result<String, MoonError> {
    let moon_home_dir = path::get_home_dir()
        .expect("Invalid home directory.")
        .join(CONFIG_DIRNAME);
    let id_path = moon_home_dir.join("id");

    if id_path.exists() {
        return fs::read(id_path);
    }

    let id = Uuid::new_v4().to_string();

    fs::create_dir_all(&moon_home_dir)?;
    fs::write(id_path, &id)?;

    Ok(id)
}

fn create_anonymous_rid(workspace_root: &Path) -> String {
    moon_utils::hash(fs::file_name(workspace_root))
}

pub async fn check_version(
    local_version_str: &str,
) -> Result<(String, bool), Box<dyn Error + Send + Sync>> {
    let moon_dir = fs::find_upwards(
        CONFIG_DIRNAME,
        env::current_dir().expect("Invalid working directory."),
    );

    if is_test_env() || proto::is_offline() || moon_dir.is_none() {
        return Ok((local_version_str.to_owned(), false));
    }

    let moon_dir = moon_dir.unwrap();
    let workspace_root = moon_dir.parent().unwrap().to_path_buf();
    let check_state_path = moon_dir.join("cache/states/versionCheck.json");
    let now = SystemTime::now();

    // Only check once every 12 hours
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
        .header("X-Moon-UID", load_or_create_anonymous_uid()?)
        .header("X-Moon-RID", create_anonymous_rid(&workspace_root))
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
