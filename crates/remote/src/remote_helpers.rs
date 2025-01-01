use bazel_remote_apis::build::bazel::remote::execution::v2::compressor;
use moon_config::RemoteCompression;

pub fn get_compressor(compression: RemoteCompression) -> i32 {
    match compression {
        RemoteCompression::None => compressor::Value::Identity as i32,
        RemoteCompression::Zstd => compressor::Value::Zstd as i32,
    }
}
