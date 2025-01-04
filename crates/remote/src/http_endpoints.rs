use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusEndpoint {
    curr_size: usize,
    git_commit: String,
    max_size: usize,
    num_files: usize,
    num_goroutines: usize,
    reserved_size: usize,
    server_time: usize,
    uncompressed_size: usize,
}
