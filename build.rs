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

#[derive(Deserialize)]
struct NameData {
    two_letter_given_names: Vec<String>,
    uncapitalized_particles: Vec<String>,
    additional_surname_prefixes: Vec<String>,
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

    let json = read_file(&input, "build/name_data.json")?;
    let names: NameData = serde_json::from_str(&json)?;
    write_data_file(
        &output.join("two_letter_given_names.rs"),
        names
            .two_letter_given_names
            .iter()
            .flat_map(|n| [n.clone(), n.to_uppercase(), n.to_lowercase()])
            .map(|n| format!("set.insert(\"{}\");", n)),
    )?;
    // Store capitalized versions because we check after doing the initial,
    // naive capitalization; use a simple capitalization algorithm here
    // because we know the data is all simple.
    let capitalized_uncapitalized_particles = names
        .uncapitalized_particles
        .iter()
        .map(|n| format!("{}{}", n[..1].to_uppercase(), &n[1..]))
        .collect::<Vec<String>>();
    write_data_file(
        &output.join("capitalized_uncapitalized_particles.rs"),
        capitalized_uncapitalized_particles
            .iter()
            .map(|n| format!("set.insert(\"{}\");", n)),
    )?;
    let mut surname_prefixes = names.uncapitalized_particles.clone();
    surname_prefixes.extend_from_slice(capitalized_uncapitalized_particles.as_slice());
    surname_prefixes.extend_from_slice(names.additional_surname_prefixes.as_slice());
    write_data_file(
        &output.join("surname_prefixes.rs"),
        surname_prefixes
            .iter()
            .map(|n| format!("set.insert(\"{}\");", n)),
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

fn quoted_comma_separated(vs: &[String]) -> String {
    vs.iter()
        .map(|v| format!("\"{}\"", v))
        .collect::<Vec<_>>()
        .join(", ")
}
