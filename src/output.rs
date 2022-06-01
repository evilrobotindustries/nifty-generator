use crate::{Arguments, PATH_TO_STRING_MSG};
use anyhow::{Context, Result};
use log::{debug, trace};
use std::path::{Path, PathBuf};

pub(crate) fn init(args: &Arguments) -> Result<PathBuf> {
    debug!("checking output directories...");
    let output_path = init_output(&args)?;
    init_media(&args, &output_path)?;
    init_metadata(&args, &output_path)?;
    Ok(output_path)
}

fn init_media(args: &Arguments, output: &PathBuf) -> Result<()> {
    let media_path = output
        .join(&args.media)
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

fn init_metadata(args: &Arguments, output: &PathBuf) -> Result<()> {
    let metadata_path = output
        .join(&args.metadata)
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

fn init_output(args: &Arguments) -> Result<PathBuf> {
    let output = args.source.join(&args.output);
    let output_path = &output.to_str().expect(PATH_TO_STRING_MSG);
    trace!("checking output directory '{output_path}' exists...");
    if !Path::new(&output).is_dir() {
        trace!("output directory does not exist, creating...");
        std::fs::create_dir(&output)
            .with_context(|| format!("could not create output directory {output_path}"))?;
    }
    Ok(output)
}
