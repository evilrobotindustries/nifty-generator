use crate::{Arguments, PATH_TO_STRING_MSG};
use anyhow::{Context, Result};
use image::{ImageFormat, Rgba};
use indexmap::IndexMap;
use log::{debug, trace};
use serde::de::{MapAccess, Visitor};
use serde::{de, Deserialize, Deserializer};
use std::fmt::Formatter;
use std::fs::OpenOptions;
use std::io;
use std::io::ErrorKind;
use std::num::ParseIntError;
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

    // Validate all configured paths exist and return config if successful
    config.validate(&args.source)?;
    Ok(config)
}

#[derive(Deserialize)]
pub(crate) struct Config {
    pub name: String,
    pub description: String,
    pub supply: u32,
    pub external_url: Option<String>,
    pub background_color: Option<Color>,
    pub attributes: IndexMap<String, IndexMap<String, Option<AttributeOption>>>,
}

impl Config {
    pub(crate) fn validate(&self, path: &Path) -> Result<()> {
        debug!("validating configuration...");

        // Check if configured paths exists
        for attribute in self.attributes.values() {
            for value in attribute.values() {
                if let Some(value) = value {
                    match value {
                        AttributeOption::Audio { file, .. } => {
                            Self::validate_path(&path.join(file))?
                        }
                        AttributeOption::Image { file, .. } => {
                            Self::validate_path(&path.join(file))?
                        }
                        AttributeOption::Text { font, .. } => {
                            Self::validate_path(&path.join(font))?
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_path(file: &PathBuf) -> Result<()> {
        let file_path = file.to_str().expect(PATH_TO_STRING_MSG);
        trace!("checking '{file_path}' file exists...");
        if !file.is_file() {
            return Err(io::Error::new(ErrorKind::NotFound, file_path)).with_context(|| {
                format!("could not find '{file_path}' file - correct the config and try again")
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum AttributeOption {
    Audio {
        /// The path to the audio file to be used.
        file: PathBuf,
        /// The weighting for the option.
        weight: Option<f32>,
    },
    Color {
        color: Color,
        /// The weighting for the option.
        weight: Option<f32>,
    },
    Image {
        file: PathBuf,
        /// The weighting for the option.
        weight: Option<f32>,
    },
    Text {
        /// The path to the font to be used.
        font: PathBuf,
        /// The text to be used.
        text: String,
        /// The height, in pixels.
        height: f32,
        x: i32,
        y: i32,
        /// The color of the text.
        color: Color,
        /// The weighting for the option.
        weight: Option<f32>,
    },
}

impl<'de> Deserialize<'de> for AttributeOption {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct AttributeOptionVisitor;

        impl<'de> Visitor<'de> for AttributeOptionVisitor {
            type Value = AttributeOption;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("struct AttributeOption")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Self::Value, V::Error> {
                let mut color = None;
                let mut file = None;
                let mut font = None;
                let mut height = None;
                let mut text = None;
                let mut x = None;
                let mut y = None;
                let mut weight = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "color" => {
                            if color.is_some() {
                                return Err(de::Error::duplicate_field("color"));
                            }
                            let value: String = map.next_value()?;
                            color = Some(match Color::from_hex(&value) {
                                Ok(color) => Ok(color),
                                Err(e) => Err(de::Error::custom(format!(
                                    "unable to parse {value} as a hex color string: {e}",
                                ))),
                            }?);
                        }
                        "file" => {
                            if file.is_some() {
                                return Err(de::Error::duplicate_field("file"));
                            }
                            let value: String = map.next_value()?;
                            file = Some(PathBuf::from(value));
                        }
                        "font" => {
                            if font.is_some() {
                                return Err(de::Error::duplicate_field("font"));
                            }
                            font = Some(map.next_value()?);
                        }
                        "height" => {
                            if height.is_some() {
                                return Err(de::Error::duplicate_field("height"));
                            }
                            height = Some(map.next_value()?);
                        }
                        "text" => {
                            if text.is_some() {
                                return Err(de::Error::duplicate_field("text"));
                            }
                            text = Some(map.next_value()?);
                        }
                        "x" => {
                            if x.is_some() {
                                return Err(de::Error::duplicate_field("x"));
                            }
                            x = Some(map.next_value()?);
                        }
                        "y" => {
                            if y.is_some() {
                                return Err(de::Error::duplicate_field("y"));
                            }
                            y = Some(map.next_value()?);
                        }
                        "weight" => {
                            if weight.is_some() {
                                return Err(de::Error::duplicate_field("weight"));
                            }
                            let value = map.next_value()?;
                            if value < 0.0 || value > 1.0 {
                                return Err(de::Error::custom(format!(
                                    "the weight of {value} is invalid - expected a value > 0 and <= 1"
                                )));
                            }
                            weight = Some(value);
                        }
                        _ => {}
                    }
                }

                // Decide on type based on specified files
                if let Some(file) = file {
                    let extension = file.extension().map(|e| e.to_ascii_lowercase());
                    return match extension.as_ref().and_then(|e| e.to_str()) {
                        Some(extension) => {
                            if SUPPORTED_AUDIO_EXTENSIONS.contains(&extension) {
                                Ok(AttributeOption::Audio { file, weight })
                                // Use supported extensions from underlying image library
                            } else if let Some(_) = ImageFormat::from_extension(&extension) {
                                Ok(AttributeOption::Image { file, weight })
                            } else {
                                Err(de::Error::custom(format!(
                                    "file extension {extension} not supported"
                                )))
                            }
                        }
                        None => Err(de::Error::custom("no file extension")),
                    };
                } else if let Some(font) = font {
                    let text = text.ok_or_else(|| de::Error::missing_field("text"))?;
                    let height = height.ok_or_else(|| de::Error::missing_field("height"))?;
                    let x = x.ok_or_else(|| de::Error::missing_field("x"))?;
                    let y = y.ok_or_else(|| de::Error::missing_field("y"))?;
                    let color = color.ok_or_else(|| de::Error::missing_field("color"))?;
                    return Ok(AttributeOption::Text {
                        font,
                        text,
                        height,
                        x,
                        y,
                        color,
                        weight,
                    });
                } else if let Some(color) = color {
                    return Ok(AttributeOption::Color { color, weight });
                }

                Err(de::Error::custom("unable to determine attribute option"))
            }
        }

        const FIELDS: &'static [&'static str] = &["color", "file", "weight"];
        deserializer.deserialize_struct("AttributeOption", FIELDS, AttributeOptionVisitor)
    }
}

#[derive(Debug)]
pub struct Color {
    pub(crate) hex: String,
    pub(crate) rgba: Rgba<u8>,
}

impl Color {
    fn from_hex(hex: &str) -> Result<Color, ParseIntError> {
        let rgba = Rgba([
            u8::from_str_radix(&hex[1..3], 16)?,
            u8::from_str_radix(&hex[3..5], 16)?,
            u8::from_str_radix(&hex[5..7], 16)?,
            u8::from_str_radix(if hex.len() == 9 { &hex[7..9] } else { "FF" }, 16)?,
        ]);
        Ok(Color {
            hex: hex.to_string(),
            rgba,
        })
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct ColorVisitor;

        impl<'de> Visitor<'de> for ColorVisitor {
            type Value = Color;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string")
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                if !s.starts_with("#") {
                    return Err(de::Error::custom(format!(
                        "unable to parse {s} as a hex color string",
                    )));
                }

                match Color::from_hex(s) {
                    Ok(color) => Ok(color),
                    Err(e) => Err(de::Error::custom(format!(
                        "unable to parse {s} as a hex color string: {}",
                        e
                    ))),
                }
            }
        }

        deserializer.deserialize_str(ColorVisitor)
    }
}
