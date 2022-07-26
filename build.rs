use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct NickData {
    names_by_nick_prefix: HashMap<String, Vec<String>>,
    names_by_irregular_nick: HashMap<String, Vec<String>>,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const NICK_DATA_FILE: &str = "build/nick_data.json";

fn main() -> Result<()> {
    let input = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join(NICK_DATA_FILE);
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", NICK_DATA_FILE);

    write_tables(&input, &output)?;

    Ok(())
}

fn write_tables(input: &Path, output: &Path) -> Result<()> {
    let input = fs::read_to_string(input)?;
    let data = serde_json::from_str::<NickData>(&input)?;

    let mut lines: Vec<String> = vec!["{".to_string()];
    for (k, vs) in data.names_by_nick_prefix.iter() {
        let vs = vs
            .iter()
            .map(|v| format!("\"{}\"", v))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("map.insert(\"{}\", &[{}] as &[_]);", k, vs));
    }
    lines.push("}".to_string());
    fs::write(&output.join("names_by_nick_prefix.rs"), lines.join("\n"))?;

    let mut lines: Vec<String> = vec!["{".to_string()];
    for (k, vs) in data.names_by_irregular_nick.iter() {
        let vs = vs
            .iter()
            .map(|v| format!("\"{}\"", v))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("map.insert(\"{}\", &[{}] as &[_]);", k, vs));
    }
    lines.push("}".to_string());
    fs::write(&output.join("names_by_irregular_nick.rs"), lines.join("\n"))?;

    Ok(())
}
