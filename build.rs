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

#[derive(Deserialize)]
struct GenerationData {
    generation_by_suffix: HashMap<String, u8>,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let input = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());

    let json = read_file(&input, "build/nick_data.json")?;
    let nicks: NickData = serde_json::from_str(&json)?;
    write_map(
        &output.join("names_by_nick_prefix.rs"),
        &nicks.names_by_nick_prefix,
        |vs| format!("&[{}] as &[_]", quoted_comma_separated(vs)),
    )?;
    write_map(
        &output.join("names_by_irregular_nick.rs"),
        &nicks.names_by_irregular_nick,
        |vs| format!("&[{}] as &[_]", quoted_comma_separated(vs)),
    )?;

    let json = read_file(&input, "build/title_data.json")?;
    let titles: TitleData = serde_json::from_str(&json)?;
    write_map(
        &output.join("honorific_prefixes.rs"),
        &titles.honorific_prefixes,
        |v| format!("\"{}\"", v),
    )?;
    write_map(
        &output.join("honorific_suffixes.rs"),
        &titles.honorific_suffixes,
        |v| format!("\"{}\"", v),
    )?;

    let json = read_file(&input, "build/name_data.json")?;
    let names: NameData = serde_json::from_str(&json)?;
    let two_letter_given_names = names
        .two_letter_given_names
        .iter()
        .flat_map(|n| [n.clone(), n.to_uppercase(), n.to_lowercase()])
        .collect::<Vec<_>>();
    write_set(
        &output.join("two_letter_given_names.rs"),
        &two_letter_given_names,
    )?;
    // Store capitalized versions because we check after doing the initial,
    // naive capitalization; use a simple capitalization algorithm here
    // because we know the data is all simple.
    let capitalized_uncapitalized_particles = names
        .uncapitalized_particles
        .iter()
        .map(|n| format!("{}{}", n[..1].to_uppercase(), &n[1..]))
        .collect::<Vec<String>>();
    let mut particles_and_conjunctions = capitalized_uncapitalized_particles.clone();
    particles_and_conjunctions.push("E".to_string());
    particles_and_conjunctions.push("Y".to_string());
    write_set(
        &output.join("particles_and_conjunctions.rs"),
        &particles_and_conjunctions,
    )?;
    let mut surname_prefixes = names.uncapitalized_particles.clone();
    surname_prefixes.extend_from_slice(capitalized_uncapitalized_particles.as_slice());
    surname_prefixes.extend_from_slice(names.additional_surname_prefixes.as_slice());
    write_set(&output.join("surname_prefixes.rs"), &surname_prefixes)?;

    let json = read_file(&input, "build/generation_data.json")?;
    let gens: GenerationData = serde_json::from_str(&json)?;
    let generation_by_suffix = gens
        .generation_by_suffix
        .iter()
        .flat_map(|(k, v)| [(k.clone(), v), (k.to_uppercase(), v), (k.to_lowercase(), v)])
        .collect::<HashMap<_, _>>();
    write_map(
        &output.join("generation_by_suffix.rs"),
        &generation_by_suffix,
        |v| v.to_string(),
    )?;

    Ok(())
}

fn write_map<'a, K, V, F>(output: &Path, map: &'a HashMap<K, V>, transform: F) -> Result<()>
where
    K: std::ops::Deref<Target = str>,
    F: Fn(&'a V) -> String,
{
    let mut builder = phf_codegen::Map::new();
    for (k, v) in map {
        builder.entry(k.to_string(), &transform(v));
    }
    fs::write(output, format!("{}", builder.build()))?;
    Ok(())
}

fn write_set(output: &Path, set: &[String]) -> Result<()> {
    let mut builder = phf_codegen::Set::new();
    for v in set {
        builder.entry(v);
    }
    fs::write(output, format!("{}", builder.build()))?;
    Ok(())
}

fn read_file(input_dir: &Path, file_path: &str) -> Result<String> {
    println!("cargo:rerun-if-changed={}", file_path);
    let s = fs::read_to_string(input_dir.join(file_path))?;
    Ok(s)
}

fn quoted_comma_separated(vs: &[String]) -> String {
    vs.iter()
        .map(|v| format!("\"{}\"", v))
        .collect::<Vec<_>>()
        .join(", ")
}
