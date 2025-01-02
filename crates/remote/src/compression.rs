use bazel_remote_apis::build::bazel::remote::execution::v2::compressor;
use moon_config::RemoteCompression;
use std::io;

// bazel-remote uses zstd and the fastest compression: level 1
// https://github.com/buchgr/bazel-remote/blob/master/cache/disk/zstdimpl/gozstd.go#L13
// https://github.com/klauspost/compress/tree/master/zstd#status

pub fn get_compressor(compression: RemoteCompression) -> i32 {
    match compression {
        RemoteCompression::None => compressor::Value::Identity as i32,
        RemoteCompression::Zstd => compressor::Value::Zstd as i32,
    }
}

pub fn compress_blob(compression: RemoteCompression, bytes: Vec<u8>) -> Result<Vec<u8>, io::Error> {
    match compression {
        RemoteCompression::None => Ok(bytes),
        RemoteCompression::Zstd => zstd::encode_all(bytes.as_slice(), 1),
    }
}

pub fn decompress_blob(
    compression: RemoteCompression,
    bytes: Vec<u8>,
) -> Result<Vec<u8>, io::Error> {
    match compression {
        RemoteCompression::None => Ok(bytes),
        RemoteCompression::Zstd => zstd::decode_all(bytes.as_slice()),
    }
}
