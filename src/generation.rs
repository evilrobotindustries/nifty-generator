use crate::metadata::{Attribute, Metadata};
use crate::{combinations, Arguments, Config, PATH_TO_STRING_MSG};
use anyhow::{Context, Result};
use hhmmss::Hhmmss;
use image::{imageops, DynamicImage};
use log::{debug, error, info, trace};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;
use thousands::Separable;

const ID: &str = "id";

pub(crate) fn generate(args: Arguments, config: &Config) -> Result<()> {
    info!("starting nifty generation...");
    let current = Instant::now();

    let source: &Path = &args.source;
    let media: &Path = &source.join(&args.output).join(&args.media);
    let metadata: &Path = &source.join(&args.output).join(&args.metadata);
    let mut images = HashMap::new();
    let mut token_variables = HashMap::new();

    // Generate all combinations, select a random subset and then build token images and metadata
    let combinations = combinations::combinations(&config);
    let combinations_count = combinations.len();
    let combinations = crate::random::random(combinations, config.supply);
    info!(
        "randomly selected {} items from {} combinations",
        combinations.len().separate_with_commas(),
        combinations_count.separate_with_commas()
    );

    for (token, attributes) in combinations.iter().enumerate() {
        let token = token + 1;
        info!("generating nifty #{}", token);

        token_variables.insert(ID.to_string(), token.to_string());

        // Create a new image
        let mut token_image: Option<Rc<DynamicImage>> = None;
        let mut token_attributes = Vec::<Attribute>::with_capacity(attributes.len());

        // Process layers
        for (layer, (attribute, value, file)) in attributes.iter().enumerate() {
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

            // Continue when no trait
            if file.is_none() {
                continue;
            }

            // Get image and cache for subsequent use
            let path = source
                .join(&directory)
                .join(file.as_ref().expect("could not get expected file"))
                .into_os_string()
                .into_string()
                .expect(PATH_TO_STRING_MSG);
            if !images.contains_key(&path) {
                trace!("caching '{path}' for next use...");
                let image = image::open(&path).with_context(|| format!("unable to open {path}"))?;
                images.insert(path.clone(), image);
            }
            let layer_image = images.get(&path).expect("could not get cached image");

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

        // Save image to output folder
        if let Some(token_image) = token_image {
            let image_name = format!("{token}.png");
            let path = media
                .join(&image_name)
                .into_os_string()
                .into_string()
                .expect(PATH_TO_STRING_MSG);
            let token_metadata = Metadata {
                name: strfmt::strfmt(&config.token_name, &token_variables).with_context(|| {
                    "unable to name token {token} using the configured token name format"
                })?,
                description: &config.description,
                image: Path::new(
                    &media
                        .components()
                        .last()
                        .expect("could not get last component from path"),
                )
                .join(&image_name)
                .to_str()
                .expect(PATH_TO_STRING_MSG)
                .to_string(),
                external_url: None,
                attributes: token_attributes,
                background_color: config.background_color.as_deref(),
                animation_url: None,
                youtube_url: None,
            };

            debug!("saving token {token} media as '{path}'");
            if let Err(e) = token_image.save(&path) {
                error!("error saving {path}: {e}")
            }

            let path = metadata
                .join(token.to_string())
                .into_os_string()
                .into_string()
                .expect(PATH_TO_STRING_MSG);
            debug!("saving token {token} metadata as '{path}'");
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;
            if let Err(e) = serde_json::to_writer(file, &token_metadata) {
                error!("error saving {path}: {e}")
            }
        }
    }

    info!("generation completed in {}", current.elapsed().hhmmssxxx());
    Ok(())
}
