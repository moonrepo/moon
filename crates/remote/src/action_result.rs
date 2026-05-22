use bazel_remote_apis::build::bazel::remote::execution::v2::NodeProperties;
use bazel_remote_apis::google::protobuf::Timestamp;
use chrono::NaiveDateTime;
use std::fs::Metadata;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn create_timestamp(time: SystemTime) -> Option<Timestamp> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| Timestamp {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos() as i32,
        })
}

pub fn create_timestamp_from_naive(time: NaiveDateTime) -> Option<Timestamp> {
    let utc = time.and_utc();

    Some(Timestamp {
        seconds: utc.timestamp(),
        nanos: utc.timestamp_subsec_nanos() as i32,
    })
}

#[cfg(unix)]
pub fn is_file_executable(_path: &Path, metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
pub fn is_file_executable(path: &Path, _metadata: &Metadata) -> bool {
    path.extension().is_some_and(|ext| ext == "exe")
}

pub fn compute_node_properties(metadata: &Metadata) -> NodeProperties {
    let mut props = NodeProperties::default();

    if let Ok(time) = metadata.modified() {
        props.mtime = create_timestamp(time);
    }

    #[cfg(unix)]
    {
        use bazel_remote_apis::google::protobuf::UInt32Value;
        use std::os::unix::fs::PermissionsExt;

        props.unix_mode = Some(UInt32Value {
            value: metadata.permissions().mode(),
        });
    }

    props
}
