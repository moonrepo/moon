use crate::errors::ProbeError;
use crate::resolver::Resolvable;
use log::trace;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs;

#[async_trait::async_trait]
pub trait Downloadable<'tool>: Send + Sync + Resolvable<'tool> {
    /// Returns an absolute file path to the downloaded file.
    /// This may not exist, as the path is composed ahead of time.
    /// This is typically ~/.probe/temp/<file>.
    fn get_download_path(&self) -> Result<PathBuf, ProbeError>;

    /// Download the tool (as an archive) from its distribution registry
    /// into the ~/.probe/temp folder and return an absolute file path.
    /// A custom URL that points to the downloadable archive can be
    /// provided as the 2nd argument.
    async fn download(&self, to_file: &Path, from_url: Option<&str>) -> Result<(), ProbeError>;
}

pub async fn download_from_url<U, F>(url: U, dest_file: F) -> Result<(), ProbeError>
where
    U: AsRef<str>,
    F: AsRef<Path>,
{
    let url = url.as_ref();
    let dest_file = dest_file.as_ref();
    let handle_io_error =
        |e: io::Error| ProbeError::FileSystem(dest_file.to_path_buf(), e.to_string());
    let handle_http_error = |e: reqwest::Error| ProbeError::Http(url.to_owned(), e.to_string());

    trace!(
        target: "probe:downloader",
        "Downloading {} from {}",
        dest_file.to_string_lossy(),
        url
    );

    // Ensure parent directories exist
    if let Some(parent) = dest_file.parent() {
        fs::create_dir_all(parent).await.map_err(handle_io_error)?;
    }

    // Fetch the file from the HTTP source
    let response = reqwest::get(url).await.map_err(handle_http_error)?;
    let status = response.status();

    if !status.is_success() {
        return Err(ProbeError::DownloadFailed(
            url.to_owned(),
            status.to_string(),
        ));
    }

    // Write the bytes to our local file
    let mut contents = io::Cursor::new(response.bytes().await.map_err(handle_http_error)?);
    let mut file = File::create(dest_file).map_err(handle_io_error)?;

    io::copy(&mut contents, &mut file).map_err(handle_io_error)?;

    Ok(())
}
