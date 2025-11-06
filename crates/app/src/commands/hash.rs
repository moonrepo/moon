use crate::app_error::AppError;
use crate::session::MoonSession;
use clap::Args;
use moon_common::color;
use serde::{Deserialize, Serialize};
use starbase::AppResult;
use starbase_utils::{
    fs,
    json::{self, JsonValue},
};
use std::collections::BTreeMap;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct HashArgs {
    #[arg(required = true, help = "Hash to inspect")]
    hash: String,

    #[arg(help = "Another hash to diff against")]
    diff_hash: Option<String>,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip(session))]
pub async fn hash(session: MoonSession, args: HashArgs) -> AppResult {
    match &args.diff_hash {
        Some(other_hash) => diff_hashes(&session, &args.hash, other_hash, args.json).await?,
        None => view_hash(&session, &args.hash, args.json).await?,
    };

    Ok(None)
}

async fn load_hash_manifest(
    session: &MoonSession,
    partial_hash: &str,
) -> miette::Result<(String, String, JsonValue)> {
    if let Some(manifest_path) = session
        .get_cache_engine()?
        .hash
        .find_manifest_path(partial_hash)?
    {
        let hash = fs::file_name(&manifest_path).replace(".json", "");

        // Our cache is non-pretty, but we wan't to output as pretty,
        // so we need to manually convert it here!
        let data: JsonValue = json::read_file(manifest_path)?;

        return Ok((hash, json::format(&data, true)?, data));
    }

    Err(AppError::MissingHashManifest(partial_hash.to_owned()).into())
}

async fn view_hash(session: &MoonSession, partial_hash: &str, as_json: bool) -> miette::Result<()> {
    let (hash, manifest, _) = load_hash_manifest(session, partial_hash).await?;
    let console = &session.console;

    if !as_json {
        console
            .out
            .write_line(format!("Hash: {}", color::hash(hash)))?;
        console.out.write_newline()?;
    }

    console.out.write_line(manifest)?;

    Ok(())
}

#[derive(Default, Deserialize, Serialize)]
pub struct HashDiffResult {
    left: JsonValue,
    left_hash: String,
    left_diffs: BTreeMap<usize, String>,
    right: JsonValue,
    right_hash: String,
    right_diffs: BTreeMap<usize, String>,
}

async fn diff_hashes(
    session: &MoonSession,
    partial_left_hash: &str,
    partial_right_hash: &str,
    as_json: bool,
) -> miette::Result<()> {
    let (left_hash, left_manifest, left_data) =
        load_hash_manifest(session, partial_left_hash).await?;
    let (right_hash, right_manifest, right_data) =
        load_hash_manifest(session, partial_right_hash).await?;
    let console = &session.console;

    if as_json {
        let mut result = HashDiffResult {
            left: left_data,
            left_hash,
            right: right_data,
            right_hash,
            ..Default::default()
        };

        for (line, diff) in diff::lines(&left_manifest, &right_manifest)
            .into_iter()
            .enumerate()
        {
            match diff {
                diff::Result::Left(l) => {
                    result.left_diffs.insert(line, l.trim().to_owned());
                }
                diff::Result::Right(r) => {
                    result.right_diffs.insert(line, r.trim().to_owned());
                }
                _ => {}
            };
        }

        console.out.write_line(json::format(&result, true)?)?;
    } else {
        console
            .out
            .write_line(format!("Left:  {}", color::success(&left_hash)))?;
        console
            .out
            .write_line(format!("Right: {}", color::failure(&right_hash)))?;
        console.out.write_newline()?;

        let is_tty = console.out.is_terminal();

        for diff in diff::lines(&left_manifest, &right_manifest) {
            match diff {
                diff::Result::Left(l) => {
                    if is_tty {
                        console.out.write_line(color::success(l))?
                    } else {
                        console.out.write_line(format!("+{l}"))?
                    }
                }
                diff::Result::Both(l, _) => {
                    if is_tty {
                        console.out.write_line(l)?
                    } else {
                        console.out.write_line(format!(" {l}"))?
                    }
                }
                diff::Result::Right(r) => {
                    if is_tty {
                        console.out.write_line(color::failure(r))?
                    } else {
                        console.out.write_line(format!("-{r}"))?
                    }
                }
            };
        }
    }

    Ok(())
}
