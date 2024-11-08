use cached::proc_macro::cached;
use moon_lang::LockfileDependencyVersions;
use pep_508::parse;
use rustc_hash::FxHashMap;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    if let Ok(lines) = read_lines(&path) {
        for line in lines.map_while(Result::ok) {
            if let Ok(parsed) = parse(&line) {
                deps.entry(parsed.name.to_string())
                    .and_modify(|dep| {
                        dep.push(line.clone());
                    })
                    .or_insert(vec![line.clone()]);
            }
        }
    }

    Ok(deps)
}
