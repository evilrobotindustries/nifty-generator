mod caches;

use self::caches::Cache;
use crate::config::{Attribute, AttributeOption, Color};
use crate::generation::caches::{AudioCache, ColorCache, FontCache, ImageCache};
use crate::random::AttributeValue;
use crate::{metadata, Config, PATH_TO_STRING_MSG};
use anyhow::{Context, Result};
use ffmpeg_cli::{FfmpegBuilder, Parameter};
use hhmmss::Hhmmss;
use image::{imageops, DynamicImage};
use imageproc::drawing::{draw_text, text_size};
use log::{debug, error, info, trace};
use rusttype::Scale;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

const ID: &str = "id";

pub(crate) async fn generate(
    source: &PathBuf,
    output: &str,
    media: &str,
    metadata: &str,
    config: Config,
) -> Result<()> {
    // Validate the config before starting generation
    validate(&config)?;

    // Initialise generator and start
    Generator::new(source, output, media, metadata, &config)
        .start(&config)
        .await
}

pub(crate) fn validate(config: &Config) -> Result<()> {
    // Check if any audio configured
    if !config.attributes.iter().any(|a| {
        a.options
            .values()
            .any(|o| matches!(o, AttributeOption::Audio { .. }))
    }) {
        return Ok(());
    }

    // Ensure ffmpeg exists
    trace!("checking for ffmpeg...");
    if let Err(e) = std::process::Command::new("ffmpeg")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let std::io::ErrorKind::NotFound = e.kind() {
            return Err(e).with_context(|| "'ffmpeg' was not found - check your PATH");
        }
        return Err(e).with_context(|| "could not run 'ffmpeg'");
    }

    Ok(())
}

struct Generator<'a> {
    source: PathBuf,
    media: PathBuf,
    metadata: PathBuf,
    name: &'a str,
    description: &'a str,
    external_url: Option<&'a String>,
    background_color: Option<&'a Color>,
    start_token: usize,
    caches: Caches<'a>,
}

struct Caches<'a> {
    audio: AudioCache,
    color: ColorCache,
    font: FontCache<'a>,
    image: ImageCache,
}

impl<'a> Generator<'a> {
    fn new(
        source: &PathBuf,
        output: &str,
        media: &str,
        metadata: &str,
        config: &'a Config,
    ) -> Self {
        let media = source.join(output).join(media);
        let metadata = source.join(output).join(metadata);
        Self {
            source: source.clone(),
            media,
            metadata,
            name: config.name.as_ref(),
            description: config.description.as_ref(),
            external_url: config.external_url.as_ref(),
            background_color: config.background_color.as_ref(),
            start_token: config.start_token,
            caches: Caches {
                audio: AudioCache::new(),
                color: ColorCache::new(),
                font: FontCache::new(),
                image: ImageCache::new(),
            },
        }
    }

    async fn start(&mut self, config: &Config) -> Result<()> {
        // Generate the collection based on configuration
        info!("starting nifty generation...");
        let current = Instant::now();
        for (i, attributes) in crate::random::generate(&config)
            .with_context(|| "failed to generate the collection")?
            .iter()
            .enumerate()
        {
            self.generate_token(i + self.start_token, attributes)
                .await?;
        }

        info!("generation completed in {}", current.elapsed().hhmmssxxx());
        Ok(())
    }

    async fn generate_token(
        &mut self,
        token: usize,
        attributes: &Vec<(&Attribute, &AttributeValue, &AttributeOption)>,
    ) -> Result<()> {
        info!("generating nifty #{}", token);

        // Create a new image
        let mut token_attributes = Vec::new();
        let mut token_audio: Option<PathBuf> = None;
        let mut token_color: Option<&Color> = None;
        let mut token_image: Option<DynamicImage> = None;

        // Process layers
        for (layer, (attribute, value, option)) in attributes.iter().enumerate() {
            debug!(
                "processing attribute '{}' with value of '{value}' as layer {layer}",
                attribute.name
            );

            // Add attribute to resulting metadata (if applicable)
            if attribute.metadata {
                token_attributes.push(metadata::Attribute::String {
                    trait_type: &attribute.name,
                    value,
                });
            }

            match option {
                AttributeOption::Audio { file, .. } => {
                    // Save audio until the end of token generation
                    token_audio = Some(file.clone());
                    continue;
                }
                AttributeOption::Color { color, .. } => {
                    // Store color for later use (i.e. first image layer to determine width/height)
                    if token_color.is_none() {
                        token_color = Some(&color);
                    }
                }
                AttributeOption::Image { file, .. } => {
                    token_image =
                        Some(self.generate_image_layer(file, token_image, token_color)?);
                }
                AttributeOption::Text {
                    font,
                    text,
                    height,
                    x,
                    y,
                    color,
                    ..
                } => {
                    token_image = Some(self.generate_text(
                        token,
                        &mut token_image,
                        font,
                        &text,
                        height,
                        x,
                        y,
                        color,
                    )?);
                }
                AttributeOption::None { .. } => {}
            }
        }

        // Save token to output folder
        if let Some(token_image) = token_image {
            // Save image
            let image_path = self.save_image(token, token_image)?;

            // Check if video to be generated
            let video_path = if let Some(audio) = token_audio {
                Some(self.generate_video(&image_path, &audio).await?)
            } else {
                None
            };

            // Finally save metadata
            let token_color = token_color.map(|color| color.hex.as_str()).or(self
                .background_color
                .as_ref()
                .map(|color| color.hex.as_str()));
            self.save_metadata(token, token_attributes, token_color, image_path, video_path)
                .with_context(|| "unable to save token metadata")?;
        }

        Ok(())
    }

    fn generate_image_layer(
        &mut self,
        file: &PathBuf,
        mut token_image: Option<DynamicImage>,
        token_color: Option<&Color>,
    ) -> Result<DynamicImage> {
        // Get image and cache for subsequent use
        let path = self
            .source
            .join(file)
            .into_os_string()
            .into_string()
            .expect(PATH_TO_STRING_MSG);
        let layer_image = self.caches.image.get(&path)?;

        // If no existing image/color, just return the image
        if token_image.is_none() {
            match token_color {
                // Just return image as first layer
                None => return Ok(layer_image.clone()),
                // Apply a background color as first/bottom layer
                Some(color) => {
                    token_image = Some(
                        self.caches
                            .color
                            .get_color(color, layer_image.width(), layer_image.height())?
                            .clone(),
                    );
                }
            }
        }

        // Add layer to image
        let mut token_image = token_image.expect("expected an existing token image");
        imageops::overlay(&mut token_image, layer_image, 0, 0);
        Ok(token_image)
    }

    fn generate_text(
        &mut self,
        token_id: usize,
        token_image: &mut Option<DynamicImage>,
        font: &PathBuf,
        text: &&String,
        height: &f32,
        x: &i32,
        y: &i32,
        color: &Color,
    ) -> Result<DynamicImage> {
        // Load font
        let path = self
            .source
            .join(font)
            .into_os_string()
            .into_string()
            .expect(PATH_TO_STRING_MSG);
        let font = self.caches.font.get(&path)?;

        // Initialise text
        let token_variables = HashMap::from([(ID.to_string(), token_id.to_string())]);
        let text = strfmt::strfmt(&text, &token_variables)
            .expect("unable to name token {token} using the configured token external url format");

        let image = token_image.as_ref().expect(
            "an image is required before text can be written - check that the text layer is above some other image layer");

        let scale = Scale::uniform(*height);
        let text_size = text_size(scale, &font, &text);
        let x = if *x < 0 {
            (image.width() as i32 + x) - text_size.0
        } else {
            *x
        };
        Ok(DynamicImage::ImageRgba8(draw_text(
            image, color.rgba, x, *y, scale, &font, &text,
        )))
    }

    async fn generate_video(&mut self, image_path: &PathBuf, audio: &PathBuf) -> Result<PathBuf> {
        // Determine precise audio duration
        let audio_path = self
            .source
            .join(audio)
            .into_os_string()
            .into_string()
            .expect(PATH_TO_STRING_MSG);
        let mut audio_duration: Option<&Duration> = None;
        if let Some(extension) = audio.extension().and_then(|e| e.to_str()) {
            if extension == "m4a" {
                trace!("determining audio track duration for precise output...");
                // Read file to determine audio length
                audio_duration = Some(
                    self.caches
                        .audio
                        .get(&audio_path)
                        .expect("could not get cached audio"),
                );
                trace!(
                    "audio track duration is {}",
                    audio_duration.unwrap().hhmmssxxx()
                );
            }
        }

        // Build ffmpeg command
        let mut video_path = image_path.clone();
        video_path.set_extension("mp4");
        let audio_duration =
            audio_duration.map_or("".to_string(), |d| format!("{}ms", d.as_millis()));
        let mut output = ffmpeg_cli::File::new(&video_path.to_str().expect(PATH_TO_STRING_MSG))
            .option(Parameter::KeyValue("acodec", "aac"))
            .option(Parameter::KeyValue("vcodec", "libx264"))
            .option(Parameter::KeyValue("pix_fmt", "yuv420p")); // Required for compatibility
        if audio_duration != "" {
            output = output.option(Parameter::KeyValue("t", &audio_duration));
        }
        let builder = FfmpegBuilder::new()
            .stderr(Stdio::piped())
            .option(Parameter::Single("nostdin"))
            .option(Parameter::KeyValue("loop", "1"))
            .input(
                ffmpeg_cli::File::new(&image_path.to_str().expect(PATH_TO_STRING_MSG))
                    .option(Parameter::KeyValue("framerate", "1")) // Single image so only single frame
                    .option(Parameter::KeyValue("colorspace", "bt709")), // Preserve colors as best as possible
            )
            .input(ffmpeg_cli::File::new(&audio_path))
            .output(output);

        // Run ffmpeg command
        let current = Instant::now();
        trace!("generating video from image and audio...");
        let ffmpeg = builder.run().await.expect("unable to run ffmpeg");
        ffmpeg
            .process
            .wait_with_output()
            .with_context(|| "could not generate the video")?;

        trace!(
            "successfully generated {} in {}",
            video_path.to_str().expect(PATH_TO_STRING_MSG),
            current.elapsed().hhmmssxxx()
        );
        Ok(video_path)
    }

    fn save_image(&self, token: usize, token_image: DynamicImage) -> Result<PathBuf> {
        let image_name = format!("{token}.png");
        let image_path = self.media.join(&image_name);
        {
            let image_path = image_path.to_str().expect(PATH_TO_STRING_MSG);
            debug!("saving token {token} media as '{image_path}'");
            if let Err(e) = token_image.save(&image_path) {
                error!("error saving {image_path}: {e}")
            }
        }

        Ok(image_path)
    }

    fn save_metadata(
        &self,
        token: usize,
        attributes: Vec<metadata::Attribute>,
        background_color: Option<&str>,
        image_path: PathBuf,
        video_path: Option<PathBuf>,
    ) -> Result<()> {
        // Generate media paths relative to output folder
        let media = self
            .media
            .components()
            .last()
            .expect("could not get last component from path");
        let media_path = Path::new("/").join(&media);
        let image_name = image_path
            .file_name()
            .expect("could not get image file name");
        let image = media_path
            .join(image_name)
            .to_str()
            .expect(PATH_TO_STRING_MSG)
            .to_string();
        let animation_url = video_path.map(|p| {
            media_path
                .join(p.file_name().expect("could not get video file name"))
                .to_str()
                .expect(PATH_TO_STRING_MSG)
                .to_string()
        });

        // Create metadata
        let token_variables = HashMap::from([(ID.to_string(), token.to_string())]);
        let token_metadata = metadata::Metadata {
            id: token,
            name: strfmt::strfmt(&self.name, &token_variables).with_context(|| {
                "unable to name token {token} using the configured token name format"
            })?,
            description: &self.description,
            image,
            external_url: self.external_url.as_ref().map(|url| {
                strfmt::strfmt(&url, &token_variables).expect(
                    "unable to name token {token} using the configured token external url format",
                )
            }),
            attributes,
            background_color: background_color.map(|color| color.replace("#", "")),
            animation_url,
            youtube_url: None,
        };

        // Save metadata
        let metadata_path = self
            .metadata
            .join(token.to_string())
            .into_os_string()
            .into_string()
            .expect(PATH_TO_STRING_MSG);
        debug!("saving token {token} metadata as '{metadata_path}'");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&metadata_path)?;
        if let Err(e) = serde_json::to_writer(file, &token_metadata) {
            error!("error saving {metadata_path}: {e}")
        }

        Ok(())
    }
}
