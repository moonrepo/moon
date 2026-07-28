use crate::remote_error::RemoteError;
use moon_blob::{Blob, Bytes};
use moon_cache_storage::Compressor;
use moon_config::RemoteCompression;
use moon_hash::Digest;
use std::ops::Deref;

#[derive(Clone)]
pub struct CompressableBlob {
    pub inner: Blob,
    pub compression: RemoteCompression,
}

impl CompressableBlob {
    pub fn new(digest: Digest, bytes: Vec<u8>) -> Self {
        Self {
            inner: Blob::new(digest, bytes),
            compression: RemoteCompression::None,
        }
    }

    pub fn from_blob(blob: Blob) -> Self {
        Self {
            inner: blob,
            compression: RemoteCompression::None,
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> miette::Result<Self> {
        Ok(Self {
            inner: Blob::from_bytes(bytes)?,
            compression: RemoteCompression::None,
        })
    }

    // pub fn from_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
    //     Ok(Self {
    //         inner: Blob::from_file(path)?,
    //         compression: RemoteCompression::None,
    //     })
    // }

    pub fn compress(&mut self, compression: RemoteCompression) -> miette::Result<()> {
        self.compression = compression;

        match compression {
            RemoteCompression::None => {
                // N/A
            }
            RemoteCompression::Zstd => {
                self.inner.bytes =
                    Bytes::from(zstd::encode_all(self.inner.bytes.as_ref(), 1).map_err(
                        |error| RemoteError::CompressFailed {
                            format: compression,
                            error: Box::new(error),
                        },
                    )?)
            }
        };

        Ok(())
    }

    // pub fn compress_and_keep(
    //     &mut self,
    //     compression: RemoteCompression,
    // ) -> miette::Result<CompressableBlob> {
    //     let uncompressed = self.clone();

    //     self.compress(compression)?;

    //     Ok(uncompressed)
    // }

    pub fn decompress(&mut self) -> miette::Result<()> {
        match self.compression {
            RemoteCompression::None => {
                // N/A
            }
            RemoteCompression::Zstd => {
                self.inner.bytes = Bytes::from(
                    zstd::decode_all(self.inner.bytes.as_ref()).map_err(|error| {
                        RemoteError::CompressFailed {
                            format: self.compression,
                            error: Box::new(error),
                        }
                    })?,
                )
            }
        };

        self.compression = RemoteCompression::None;

        Ok(())
    }

    // pub fn decompress_and_keep(&mut self) -> miette::Result<CompressableBlob> {
    //     let compressed = self.clone();

    //     self.decompress()?;

    //     Ok(compressed)
    // }
}

impl Deref for CompressableBlob {
    type Target = Blob;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// bazel-remote uses zstd and the fastest compression: level 1
// https://github.com/buchgr/bazel-remote/blob/master/cache/disk/zstdimpl/gozstd.go#L13
// https://github.com/klauspost/compress/tree/master/zstd#status

pub fn get_acceptable_compressors(compression: RemoteCompression) -> Vec<i32> {
    let mut list = vec![Compressor::Identity as i32];

    if compression == RemoteCompression::Zstd {
        list.push(Compressor::Zstd as i32);
    };

    list
}

pub fn get_compressor(compression: RemoteCompression) -> i32 {
    match compression {
        RemoteCompression::None => Compressor::Identity as i32,
        RemoteCompression::Zstd => Compressor::Zstd as i32,
    }
}

pub fn get_compression_from_code(compressor: i32) -> RemoteCompression {
    match compressor {
        zstd if zstd == Compressor::Zstd as i32 => RemoteCompression::Zstd,
        _ => RemoteCompression::None,
    }
}
