use anyhow::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

pub(crate) mod ledger;
pub(crate) mod node;
pub(crate) mod utils;
pub(crate) mod wallets;

fn read_toml(path: &PathBuf) -> Result<String> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut toml_str = String::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim_start().starts_with('#') {
            toml_str.push_str(&line);
            toml_str.push('\n');
        }
    }
    Ok(toml_str)
}
