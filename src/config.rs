use crate::{Arguments, PATH_TO_STRING_MSG};
use anyhow::{Context, Result};
use image::ImageFormat;
use log::{debug, trace};
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::fs::OpenOptions;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const SUPPORTED_AUDIO_EXTENSIONS: [&str; 5] = ["aac", "flac", "m4a", "mp3", "wav"];

pub(crate) fn load(args: &Arguments) -> Result<Config> {
    let config = args.source.join(&args.config);
    let config_path = &config.to_str().expect(PATH_TO_STRING_MSG);
    debug!("loading configuration from '{config_path}'");
    let file = OpenOptions::new()
        .read(true)
        .open(&config)
        .with_context(|| format!("failed to load configuration from {config_path}"))?;
    let mut config: Config = serde_json::from_reader(file)
        .with_context(|| format!("failed to deserialize configuration file from {config_path}"))?;

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
    pub external_url: Option<String>,
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
            for value in attribute.options.values() {
                if let Some(value) = value {
                    // Check if file exists
                    Self::validate_file(&directory, &value.file())?;
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
    pub options: HashMap<String, Option<MediaType>>,
}

#[derive(Debug)]
pub enum MediaType {
    Image(PathBuf, ImageFormat),
    Audio(PathBuf),
}

impl MediaType {
    pub(crate) fn file(&self) -> &PathBuf {
        match self {
            MediaType::Image(file, ..) => file,
            MediaType::Audio(file) => file,
        }
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct MediaTypeVisitor;

        impl<'de> Visitor<'de> for MediaTypeVisitor {
            type Value = MediaType;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string")
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                let path = PathBuf::from(s);
                let extension = path.extension().map(|e| e.to_ascii_lowercase());
                return match extension.as_ref().and_then(|e| e.to_str()) {
                    Some(extension) => {
                        if SUPPORTED_AUDIO_EXTENSIONS.contains(&extension) {
                            Ok(MediaType::Audio(path))
                        // Use supported extensions from underlying image library
                        } else if let Some(format) = ImageFormat::from_extension(&extension) {
                            Ok(MediaType::Image(path, format))
                        } else {
                            Err(de::Error::custom(format!(
                                "file extension {extension} not supported"
                            )))
                        }
                    }
                    None => Err(de::Error::custom("no file extension")),
                };
            }
        }

        deserializer.deserialize_str(MediaTypeVisitor)
    }
}
