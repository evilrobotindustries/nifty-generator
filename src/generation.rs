use self::caches::Cache;
use crate::config::MediaType;
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

        for (token, attributes) in randomised.iter().enumerate() {
            let token = token + 1;
            info!("generating nifty #{}", token);

            // Create a new image
            let mut token_image: Option<Rc<DynamicImage>> = None;
            let mut token_attributes = Vec::<Attribute>::with_capacity(attributes.len());
            let mut audio: Option<(PathBuf, MediaType)> = None;

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

                // Continue when no value
                match media_type {
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
                            MediaType::Image(file) => {
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
                                    token_image = Some(Rc::new(layer_image.clone()));
                                    continue;
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
                self.save_metadata(token, token_attributes, image_path, video_path)
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
        let audio_file = audio.1.file();
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
            .option(Parameter::KeyValue("pix_fmt", "yuv420p"));
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
            .input(ffmpeg_cli::File::new(&image_path))
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
        token_attributes: Vec<Attribute>,
        image_path: PathBuf,
        video_path: Option<PathBuf>,
    ) -> Result<()> {
        let media = self
            .media
            .components()
            .last()
            .expect("could not get last component from path");
        let media_path = Path::new(&media);
        let token_variables = HashMap::from([(ID.to_string(), token.to_string())]);
        let token_metadata = Metadata {
            name: strfmt::strfmt(&self.config.token_name, &token_variables).with_context(|| {
                "unable to name token {token} using the configured token name format"
            })?,
            description: &self.config.description,
            image: media_path
                .join(
                    image_path
                        .file_name()
                        .expect("could not get image file name"),
                )
                .to_str()
                .expect(PATH_TO_STRING_MSG)
                .to_string(),
            external_url: None,
            attributes: token_attributes,
            background_color: self.config.background_color.as_deref(),
            animation_url: video_path.map(|p| {
                media_path
                    .join(p.file_name().expect("could not get video file name"))
                    .to_str()
                    .expect(PATH_TO_STRING_MSG)
                    .to_string()
            }),
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

mod caches {

    use anyhow::{Context, Result};
    use image::DynamicImage;
    use log::trace;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;
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
}
