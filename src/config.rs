use crate::{Arguments, PATH_TO_STRING_MSG};
use anyhow::{Context, Result};
use log::{debug, trace};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub(crate) fn load(args: &Arguments) -> Result<Config> {
    let config = args.source.join(&args.config);
    let config_path = &config.to_str().expect(PATH_TO_STRING_MSG);
    debug!("loading configuration from '{config_path}'");
    let file = OpenOptions::new()
        .read(true)
        .open(&config)
        .with_context(|| format!("failed to load configuration from {config_path}"))?;
    let mut config: Config = serde_json::from_reader(file)?;

    // Reverse the attributes (layers)
    config.attributes.reverse();

    // Validate all configured directories/files exist and return config if successful
    config.validate(&args.source)?;
    Ok(config)
}

#[derive(Deserialize)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub supply: u32,
    pub token_name: String,
    pub background_color: Option<String>,
    pub attributes: Vec<Attribute>,
}

impl Config {
    pub(crate) fn validate(&self, path: &Path) -> Result<()> {
        debug!("validating configuration...");

        // Check if configured directories exists
        for attribute in &self.attributes {
            // Check if directory exists
            let directory = Self::validate_directory(path, &attribute)?;

            // Check if configured files exist
            for file in attribute.options.values() {
                if let Some(file) = file {
                    // Check if file exists
                    Self::validate_file(&directory, &file)?;
                }
            }
        }

        Ok(())
    }

    fn validate_file(directory: &PathBuf, file: &&PathBuf) -> Result<()> {
        let file = directory.join(&file);
        let file_path = file.to_str().expect(PATH_TO_STRING_MSG);
        trace!("checking '{file_path}' file exists...");
        if !file.is_file() {
            return Err(io::Error::new(ErrorKind::NotFound, file_path)).with_context(|| {
                format!("could not find '{file_path}' file - correct the config and try again")
            });
        }
        Ok(())
    }

    fn validate_directory(path: &Path, attribute: &&Attribute) -> Result<PathBuf> {
        let directory = path.join(&attribute.directory);
        let directory_path = directory.to_str().expect(PATH_TO_STRING_MSG);
        trace!("checking '{directory_path}' directory exists...");
        if !directory.is_dir() {
            return Err(io::Error::new(ErrorKind::NotFound, directory_path)).with_context(|| {
                format!(
                    "could not find '{directory_path}' directory - correct the config and try again"
                )
            });
        }
        Ok(directory)
    }
}

#[derive(Deserialize, Debug)]
pub struct Attribute {
    pub name: String,
    pub directory: PathBuf,
    pub options: HashMap<String, Option<PathBuf>>,
}
