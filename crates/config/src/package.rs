// package.json

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

// Only add fields when we need them
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct PackageJson {
    pub name: String,

    pub engines: Option<HashMap<String, String>>,

    pub version: Option<String>,
}

impl PackageJson {
    pub fn load(path: &Path) -> Result<PackageJson, io::Error> {
        let reader = io::BufReader::new(fs::File::open(path)?);
        let json: PackageJson = serde_json::from_reader(reader)?;

        Ok(json)
    }

    pub fn save(path: &Path, json: &PackageJson) -> Result<(), io::Error> {
        let writer = io::BufWriter::new(fs::File::open(path)?);

        serde_json::to_writer_pretty(writer, json)?;

        // TODO savee to file?

        Ok(())
    }
}
