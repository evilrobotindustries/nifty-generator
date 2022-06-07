mod caches;

use self::caches::Cache;
use crate::config::{Color, MediaType};
use crate::metadata::{Attribute, Metadata};
use crate::{combinations, Arguments, Config, PATH_TO_STRING_MSG};
use anyhow::{Context, Result};
use ffmpeg_cli::{FfmpegBuilder, Parameter};
use hhmmss::Hhmmss;
use image::{imageops, DynamicImage};
use log::{debug, error, info, trace};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::rc::Rc;
use std::time::{Duration, Instant};
use thousands::Separable;

const ID: &str = "id";

pub(crate) async fn generate(args: Arguments, config: Config) -> Result<()> {
    Generator::new(args, config).start().await
}

struct Generator {
    source: PathBuf,
    media: PathBuf,
    metadata: PathBuf,
    config: Config,
}

impl Generator {
    fn new(args: Arguments, config: Config) -> Self {
        let source = args.source;
        let media = source.join(&args.output).join(&args.media);
        let metadata = source.join(&args.output).join(&args.metadata);
        Self {
            source,
            media,
            metadata,
            config,
        }
    }

    async fn start(&mut self) -> Result<()> {
        info!("starting nifty generation...");
        let current = Instant::now();

        // Generate all combinations, select a random subset and then build token images and metadata
        let combinations = combinations::combinations(&self.config);
        let combinations_count = combinations.len();
        let randomised = crate::random::random(combinations, self.config.supply);
        info!(
            "randomly selected {} items from {} combinations",
            randomised.len().separate_with_commas(),
            combinations_count.separate_with_commas()
        );

        let mut audio_cache = caches::AudioCache::new();
        let mut image_cache = caches::ImageCache::new();
        let mut background_color_cache = caches::ColorCache::new();

        for (token, attributes) in randomised.iter().enumerate() {
            let token = token + 1;
            info!("generating nifty #{}", token);

            // Create a new image
            let mut token_image: Option<Rc<DynamicImage>> = None;
            let mut token_attributes = Vec::<Attribute>::with_capacity(attributes.len());
            let mut audio: Option<(PathBuf, MediaType)> = None;
            let mut token_color: Option<&Color> = None;

            // Process layers
            for (layer, (attribute, value, media_type)) in attributes.iter().enumerate() {
                let directory = attribute.directory.to_str().expect(PATH_TO_STRING_MSG);
                debug!(
                "processing attribute '{}' with value of '{value}' from directory '{directory}' as layer {layer}",
                attribute.name
            );
                // Add attribute
                token_attributes.push(Attribute::String {
                    trait_type: &attribute.name,
                    value,
                });

                match media_type {
                    // Continue when no value
                    None => continue,
                    Some(media_type) => {
                        match media_type {
                            MediaType::Audio(file) => {
                                // Save audio until the end of token generation
                                audio = Some((
                                    PathBuf::from(directory),
                                    MediaType::Audio(file.clone()),
                                ));
                                continue;
                            }
                            MediaType::Color(color) => {
                                // Store color for later use (i.e. first image layer to determine width/height)
                                if token_color.is_none() {
                                    token_color = Some(color);
                                }
                            }
                            MediaType::Image(file, ..) => {
                                // Get image and cache for subsequent use
                                let path = self
                                    .source
                                    .join(&directory)
                                    .join(file)
                                    .into_os_string()
                                    .into_string()
                                    .expect(PATH_TO_STRING_MSG);
                                let layer_image = image_cache.get(&path)?;

                                // Set token image if first layer
                                if token_image.is_none() {
                                    // Check if we should apply a background color as first layer
                                    if let Some(color) = token_color {
                                        token_image = Some(Rc::new(
                                            background_color_cache
                                                .get_color(
                                                    color,
                                                    layer_image.width(),
                                                    layer_image.height(),
                                                )?
                                                .clone(),
                                        ));
                                    } else {
                                        token_image = Some(Rc::new(layer_image.clone()));
                                        continue;
                                    }
                                }

                                // Add layer to image
                                let token_image = Rc::get_mut(
                                    token_image
                                        .as_mut()
                                        .expect("expected an existing token image"),
                                )
                                .expect("expected an existing image");
                                imageops::overlay(token_image, layer_image, 0, 0);
                            }
                        }
                    }
                }
            }

            // Save token to output folder
            if let Some(token_image) = token_image {
                // Save image
                let image_path = self.save_image(token, token_image)?;

                // Check if video to be generated
                let video_path = if let Some(audio) = audio {
                    Some(
                        self.generate_video(&image_path, audio, &mut audio_cache)
                            .await?,
                    )
                } else {
                    None
                };

                // Finally save metadata
                let token_color = token_color.map(|color| color.hex.as_str()).or(self
                    .config
                    .background_color
                    .as_ref()
                    .map(|color| color.hex.as_str()));
                self.save_metadata(token, token_attributes, token_color, image_path, video_path)
                    .with_context(|| "unable to save token metadata")?;
            }
        }

        info!("generation completed in {}", current.elapsed().hhmmssxxx());
        Ok(())
    }

    async fn generate_video(
        &self,
        image_path: &PathBuf,
        audio: (PathBuf, MediaType),
        cache: &mut caches::AudioCache,
    ) -> Result<PathBuf> {
        let audio_file = audio.1.file().expect("expected an audio file path");
        let audio_path = self
            .source
            .join(audio.0)
            .join(audio_file)
            .into_os_string()
            .into_string()
            .expect(PATH_TO_STRING_MSG);
        let mut audio_duration: Option<&Duration> = None;
        if let Some(extension) = audio_file.extension().and_then(|e| e.to_str()) {
            if extension == "m4a" {
                trace!("determining audio track duration for precise output...");
                // Read file to determine audio length
                audio_duration = Some(cache.get(&audio_path).expect("could not get cached audio"));
                trace!(
                    "audio track duration is {}",
                    audio_duration.unwrap().hhmmssxxx()
                );
            }
        }

        // "ffmpeg -loop 1 -i 0.png -i 0.m4a -c:v libx264 -c:a aac -pix_fmt yuv420p -t 6098ms -y out.mp4"
        let mut video_path = image_path.clone();
        video_path.set_extension("mp4");
        let audio_duration =
            audio_duration.map_or("".to_string(), |d| format!("{}ms", d.as_millis()));
        let output_path = &video_path
            .clone()
            .into_os_string()
            .into_string()
            .expect(PATH_TO_STRING_MSG);
        let mut output = ffmpeg_cli::File::new(&output_path)
            .option(Parameter::KeyValue("acodec", "aac"))
            .option(Parameter::KeyValue("vcodec", "libx264"))
            .option(Parameter::KeyValue("pix_fmt", "yuv420p")); // Required for compatibility
        if audio_duration != "" {
            output = output.option(Parameter::KeyValue("t", &audio_duration));
        }

        let image_path = image_path
            .clone()
            .into_os_string()
            .into_string()
            .expect(PATH_TO_STRING_MSG);
        let builder = FfmpegBuilder::new()
            .stderr(Stdio::piped())
            .option(Parameter::Single("nostdin"))
            .option(Parameter::KeyValue("loop", "1"))
            .input(
                ffmpeg_cli::File::new(&image_path)
                    .option(Parameter::KeyValue("framerate", "1")) // Single image so only single frame
                    .option(Parameter::KeyValue("colorspace", "bt709")), // Preserve colors as best as possible
            )
            .input(ffmpeg_cli::File::new(&audio_path))
            .output(output);

        let current = Instant::now();
        trace!("generating video from image and audio...");
        let ffmpeg = builder.run().await.expect("unable to run ffmpeg");
        ffmpeg
            .process
            .wait_with_output()
            .with_context(|| "could not generate the video")?;

        trace!(
            "successfully generated {} in {}",
            video_path
                .clone()
                .into_os_string()
                .into_string()
                .expect(PATH_TO_STRING_MSG),
            current.elapsed().hhmmssxxx()
        );
        Ok(video_path)
    }

    fn save_image(&self, token: usize, token_image: Rc<DynamicImage>) -> Result<PathBuf> {
        let image_name = format!("{token}.png");
        let image_path = self.media.join(&image_name);
        {
            let image_path = image_path
                .clone()
                .into_os_string()
                .into_string()
                .expect(PATH_TO_STRING_MSG);
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
        attributes: Vec<Attribute>,
        background_color: Option<&str>,
        image_path: PathBuf,
        video_path: Option<PathBuf>,
    ) -> Result<()> {
        // Generate relative media paths
        let media = self
            .media
            .components()
            .last()
            .expect("could not get last component from path");
        let media_path = Path::new(&media);
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
        let token_metadata = Metadata {
            id: token,
            name: strfmt::strfmt(&self.config.token_name, &token_variables).with_context(|| {
                "unable to name token {token} using the configured token name format"
            })?,
            description: &self.config.description,
            image,
            external_url: self.config.external_url.as_ref().map(|url| {
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
