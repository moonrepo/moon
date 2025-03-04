use crate::fs_digest::create_digest;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{Digest, compressor};
use moon_config::RemoteCompression;

#[derive(Clone)]
pub struct Blob {
    pub bytes: Vec<u8>,
    pub compressed: RemoteCompression,
    pub digest: Digest,
}

impl From<Vec<u8>> for Blob {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            digest: create_digest(&bytes),
            compressed: RemoteCompression::None,
            bytes,
        }
    }
}

impl Blob {
    pub fn new(digest: Digest, bytes: Vec<u8>) -> Self {
        Self {
            digest,
            compressed: RemoteCompression::None,
            bytes,
        }
    }

    pub fn compress(&mut self, compression: RemoteCompression) -> miette::Result<()> {
        self.compressed = compression;

        match compression {
            RemoteCompression::None => {
                // N/A
            }
            RemoteCompression::Zstd => {
                self.bytes = zstd::encode_all(self.bytes.as_slice(), 1).map_err(|error| {
                    RemoteError::CompressFailed {
                        format: compression,
                        error: Box::new(error),
                    }
                })?
            }
        };

        Ok(())
    }

    pub fn compress_and_keep(&mut self, compression: RemoteCompression) -> miette::Result<Blob> {
        let uncompressed = self.clone();

        self.compress(compression)?;

        Ok(uncompressed)
    }

    pub fn decompress(&mut self) -> miette::Result<()> {
        match self.compressed {
            RemoteCompression::None => {
                // N/A
            }
            RemoteCompression::Zstd => {
                self.bytes = zstd::decode_all(self.bytes.as_slice()).map_err(|error| {
                    RemoteError::CompressFailed {
                        format: self.compressed,
                        error: Box::new(error),
                    }
                })?
            }
        };

        self.compressed = RemoteCompression::None;

        Ok(())
    }

    pub fn decompress_and_keep(&mut self) -> miette::Result<Blob> {
        let compressed = self.clone();

        self.decompress()?;

        Ok(compressed)
    }
}

// bazel-remote uses zstd and the fastest compression: level 1
// https://github.com/buchgr/bazel-remote/blob/master/cache/disk/zstdimpl/gozstd.go#L13
// https://github.com/klauspost/compress/tree/master/zstd#status

pub fn get_acceptable_compressors(compression: RemoteCompression) -> Vec<i32> {
    let mut list = vec![compressor::Value::Identity as i32];

    if compression == RemoteCompression::Zstd {
        list.push(compressor::Value::Zstd as i32);
    };

    list
}

pub fn get_compressor(compression: RemoteCompression) -> i32 {
    match compression {
        RemoteCompression::None => compressor::Value::Identity as i32,
        RemoteCompression::Zstd => compressor::Value::Zstd as i32,
    }
}

#[allow(dead_code)]
pub fn get_compression_from_value(compressor: compressor::Value) -> RemoteCompression {
    match compressor {
        compressor::Value::Zstd => RemoteCompression::Zstd,
        _ => RemoteCompression::None,
    }
}

pub fn get_compression_from_code(compressor: i32) -> RemoteCompression {
    match compressor {
        zstd if zstd == compressor::Value::Zstd as i32 => RemoteCompression::Zstd,
        _ => RemoteCompression::None,
    }
}
