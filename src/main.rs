use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use walkdir::WalkDir;

struct Crate {
    name: String,
    version: String,
}

impl Crate {
    pub fn new(name: String, version: String) -> Self {
        Self { name, version }
    }
}

#[derive(Parser)]
struct Args {
    vendor_path: String,
    mirror_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    let vendors = get_vendors(&args.vendor_path).expect("unable to parse vendor path");

    download_and_save(&args.mirror_path, vendors).expect("unable to download crates");
}

pub fn get_crate_path(
    mirror_path: &Path,
    crate_name: &str,
    crate_version: &str,
) -> Option<PathBuf> {
    let crate_path = match crate_name.len() {
        1 => PathBuf::from("1"),
        2 => PathBuf::from("2"),
        3 => PathBuf::from("3"),
        n if n >= 4 => {
            let first_two = crate_name.get(0..2)?;
            let second_two = crate_name.get(2..4)?;
            [first_two, second_two].iter().collect()
        }
        _ => return None,
    };

    Some(
        mirror_path
            .join("crates")
            .join(crate_path)
            .join(crate_name)
            .join(crate_version)
            .join(format!("{}-{}.crate", crate_name, crate_version)),
    )
}

fn get_vendors(vendor_path: &str) -> anyhow::Result<Vec<Crate>> {
    // Walk output generated by cargo-vendor to grab all crates that are needed for this projects
    // compilation
    let mut vendors = vec![];
    for entry in WalkDir::new(vendor_path).min_depth(1).max_depth(2) {
        let path = entry.as_ref().unwrap().path();
        if path.file_name() == Some(OsStr::new("Cargo.toml")) {
            let s = fs::read_to_string(entry?.path())?;
            let crate_toml = s.parse::<toml::Value>()?;
            if let toml::Value::Table(crate_f) = crate_toml {
                let name = crate_f["package"]["name"].to_string().replace('\"', "");
                let version = crate_f["package"]["version"].to_string().replace('\"', "");
                vendors.push(Crate::new(name, version));
            }
        }
    }

    Ok(vendors)
}

fn download_and_save(mirror_path: &Path, vendors: Vec<Crate>) -> anyhow::Result<()> {
    // Download all crate files and put into spots that are expected by cargo from crates.io
    // TODO: check if crate file already exists
    // TODO: async downloading
    for Crate { name, version } in vendors {
        let url = format!(
            "https://static.crates.io/crates/{}/{}-{}.crate",
            name, name, version
        );
        println!("Downloading: {}", url);
        let response = reqwest::blocking::get(url)?;

        let crate_path = get_crate_path(mirror_path, &name, &version).unwrap();
        let mut dir_path = crate_path.clone();
        dir_path.pop();
        fs::create_dir_all(dir_path)?;
        fs::write(crate_path, response.text()?.as_bytes())?;
    }

    Ok(())
}
