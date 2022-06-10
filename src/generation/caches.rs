use crate::config::Color;
use anyhow::{Context, Result};
use image::{DynamicImage, ImageBuffer};
use log::trace;
use rusttype::Font;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::time::Duration;

pub(crate) trait Cache<T> {
    fn get(&mut self, key: &str) -> Result<&T>;
}

pub(crate) struct ImageCache(HashMap<String, DynamicImage>);

impl ImageCache {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Cache<DynamicImage> for ImageCache {
    fn get(&mut self, key: &str) -> Result<&DynamicImage> {
        if !self.0.contains_key(key) {
            trace!("caching '{key}' for next use...");
            let image = image::open(&key).with_context(|| format!("unable to open {key}"))?;
            self.0.insert(key.to_string(), image);
        }
        Ok(self.0.get(key).expect("could not get cached image"))
    }
}

pub(crate) struct AudioCache(HashMap<String, Duration>);

impl AudioCache {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Cache<Duration> for AudioCache {
    fn get(&mut self, key: &str) -> Result<&Duration> {
        if !self.0.contains_key(key) {
            let file = File::open(key.clone()).with_context(|| "error opening audio file")?;
            let size = file
                .metadata()
                .with_context(|| format!("unable to retrieve metadata for '{key}'"))?
                .len();
            let reader = BufReader::new(file);
            let reader = mp4::Mp4Reader::read_header(reader, size)?;
            self.0.insert(key.to_string(), reader.duration());
        }
        Ok(self.0.get(key).expect("could not get cached audio"))
    }
}

pub(crate) struct ColorCache(HashMap<String, DynamicImage>);
impl ColorCache {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn get_color(
        &mut self,
        color: &Color,
        width: u32,
        height: u32,
    ) -> Result<&DynamicImage> {
        let key = format!("{} {width}x{height}", color.hex);
        if !self.0.contains_key(&key) {
            trace!("caching '{key}' for next use...");
            let buffer = ImageBuffer::from_pixel(width, height, color.rgba.clone());
            let image = DynamicImage::ImageRgba8(buffer);
            self.0.insert(key.clone(), image);
        }
        Ok(self.0.get(&key).expect("could not get cached image"))
    }
}

pub(crate) struct FontCache<'a>(HashMap<String, Font<'a>>);

impl<'a> FontCache<'a> {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }
}

impl<'a> Cache<Font<'a>> for FontCache<'a> {
    fn get(&mut self, key: &str) -> Result<&Font<'a>> {
        if !self.0.contains_key(key) {
            let file = std::fs::File::open(&key).expect("could not open font file");
            let mut reader = std::io::BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;
            let font = Font::try_from_vec(buffer)
                .with_context(|| "unable to create font from file data")?;

            self.0.insert(key.to_string(), font);
        }
        Ok(self.0.get(key).expect("could not get cached font"))
    }
}
