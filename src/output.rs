use crate::PATH_TO_STRING_MSG;
use anyhow::{Context, Result};
use log::{debug, trace, warn};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(crate) fn init(source: &PathBuf, output: &str, media: &str, metadata: &str) -> Result<PathBuf> {
    debug!("checking output directories...");
    let output_path = init_output(source, output)?;
    init_media(&output_path, media)?;
    init_metadata(&output_path, metadata)?;
    Ok(output_path)
}

fn init_media(output: &PathBuf, media: &str) -> Result<()> {
    let media_path = output
        .join(media)
        .into_os_string()
        .into_string()
        .expect(PATH_TO_STRING_MSG);
    trace!("checking media output directory '{media_path}' exists...");
    if !Path::new(&media_path).is_dir() {
        trace!("media output directory does not exist, creating...");
        std::fs::create_dir(&media_path)
            .with_context(|| format!("could not create media output directory {media_path}"))?;
    }
    Ok(())
}

fn init_metadata(output: &PathBuf, metadata: &str) -> Result<()> {
    let metadata_path = output
        .join(metadata)
        .into_os_string()
        .into_string()
        .expect(PATH_TO_STRING_MSG);
    trace!("checking metadata output directory '{metadata_path}' exists...");
    if !Path::new(&metadata_path).is_dir() {
        debug!("metadata output directory does not exist, creating...");
        std::fs::create_dir(&metadata_path).with_context(|| {
            format!("could not create metadata output directory {metadata_path}")
        })?;
    }
    Ok(())
}

fn init_output(source: &PathBuf, output: &str) -> Result<PathBuf> {
    let output = source.join(output);
    let output_path = &output.to_str().expect(PATH_TO_STRING_MSG);
    trace!("checking output directory '{output_path}' exists...");

    if Path::new(&output).is_dir() {
        // Clear output as config may have changed
        warn!("output directory '{output_path}' already exists and needs to be cleared: press enter when ready to continue...");
        let mut buffer: Vec<u8> = vec![];
        std::io::stdin().read(&mut *buffer)?;
        std::fs::remove_dir_all(&output)
            .with_context(|| format!("could not create output directory {output_path}"))?;
    }

    std::fs::create_dir(&output)
        .with_context(|| format!("could not create output directory {output_path}"))?;
    Ok(output)
}
