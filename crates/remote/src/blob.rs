use crate::fs_digest::create_digest;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{compressor, Digest};
use moon_config::RemoteCompression;

#[derive(Clone)]
pub struct Blob {
    pub bytes: Vec<u8>,
    pub compressable: bool,
    pub digest: Digest,
}

impl Blob {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            digest: create_digest(&bytes),
            compressable: true,
            bytes,
        }
    }

    pub fn compress(&mut self, compression: RemoteCompression) -> miette::Result<()> {
        if !self.compressable {
            return Ok(());
        }

        self.bytes = match compression {
            RemoteCompression::None => {
                return Ok(());
            }
            RemoteCompression::Zstd => {
                zstd::encode_all(self.bytes.as_slice(), 1).map_err(|error| {
                    RemoteError::CompressFailed {
                        format: compression,
                        error: Box::new(error),
                    }
                })?
            }
        };

        self.digest = create_digest(&self.bytes);

        Ok(())
    }

    pub fn compress_and_keep(&mut self, compression: RemoteCompression) -> miette::Result<Blob> {
        let uncompressed = self.clone();

        self.compress(compression)?;

        Ok(uncompressed)
    }

    pub fn decompress(&mut self, compression: RemoteCompression) -> miette::Result<()> {
        if !self.compressable {
            return Ok(());
        }

        self.bytes = match compression {
            RemoteCompression::None => {
                return Ok(());
            }
            RemoteCompression::Zstd => {
                zstd::decode_all(self.bytes.as_slice()).map_err(|error| {
                    RemoteError::CompressFailed {
                        format: compression,
                        error: Box::new(error),
                    }
                })?
            }
        };

        self.digest = create_digest(&self.bytes);

        Ok(())
    }

    pub fn decompress_and_keep(&mut self, compression: RemoteCompression) -> miette::Result<Blob> {
        let compressed = self.clone();

        self.decompress(compression)?;

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
