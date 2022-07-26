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

#[derive(Deserialize)]
struct TitleData {
    honorific_prefixes: HashMap<String, String>,
    honorific_suffixes: HashMap<String, String>,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let input = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());

    let json = read_file(&input, "build/nick_data.json")?;
    let nicks: NickData = serde_json::from_str(&json)?;
    write_data_file(
        &output.join("names_by_nick_prefix.rs"),
        nicks.names_by_nick_prefix.iter().map(|(k, vs)| {
            format!(
                "map.insert(\"{}\", &[{}] as &[_]);",
                k,
                quoted_comma_separated(vs)
            )
        }),
    )?;
    write_data_file(
        &output.join("names_by_irregular_nick.rs"),
        nicks.names_by_irregular_nick.iter().map(|(k, vs)| {
            format!(
                "map.insert(\"{}\", &[{}] as &[_]);",
                k,
                quoted_comma_separated(vs)
            )
        }),
    )?;

    let json = read_file(&input, "build/title_data.json")?;
    let titles: TitleData = serde_json::from_str(&json)?;
    write_data_file(
        &output.join("honorific_prefixes.rs"),
        titles
            .honorific_prefixes
            .iter()
            .map(|(k, v)| format!("map.insert(\"{}\", \"{}\");", k, v)),
    )?;
    write_data_file(
        &output.join("honorific_suffixes.rs"),
        titles
            .honorific_suffixes
            .iter()
            .map(|(k, v)| format!("map.insert(\"{}\", \"{}\");", k, v)),
    )?;

    Ok(())
}

fn read_file(input_dir: &Path, file_path: &str) -> Result<String> {
    println!("cargo:rerun-if-changed={}", file_path);
    let s = fs::read_to_string(input_dir.join(file_path))?;
    Ok(s)
}

fn write_data_file<I>(output: &Path, data: I) -> Result<()>
where
    I: Iterator<Item = String>,
{
    let mut lines = vec!["{".to_string()];
    lines.extend(data);
    lines.push("}".to_string());
    fs::write(&output, lines.join("\n"))?;
    Ok(())
}

fn quoted_comma_separated(vs: &Vec<String>) -> String {
    vs.iter()
        .map(|v| format!("\"{}\"", v))
        .collect::<Vec<_>>()
        .join(", ")
}
