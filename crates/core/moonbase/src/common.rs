use serde::{Deserialize, Serialize};

pub const LOG_TARGET: &str = "moonbase";

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Response<T> {
    Failure { message: String, status: usize },
    Success(T),
}
