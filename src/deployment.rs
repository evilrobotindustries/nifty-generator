use crate::PATH_TO_STRING_MSG;
use anyhow::{Context, Error, Result};
use log::trace;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use url::{ParseError, Url};

pub(crate) fn deploy(source: &PathBuf, output: &str, metadata: &str, base_uri: &Url) -> Result<()> {
    let metadata_path = source.join(output).join(metadata);

    for file in fs::read_dir(&metadata_path).with_context(|| {
        format!(
            "unable to read metadata from {}",
            &metadata_path.to_str().expect(PATH_TO_STRING_MSG)
        )
    })? {
        // Read metadata, amending image and animation_url if values present
        let path = file?.path();
        trace!(
            "reading metadata from '{}'...",
            path.to_str().expect(PATH_TO_STRING_MSG)
        );
        let file = fs::File::open(&path).with_context(|| {
            format!(
                "unable to read metadata from {}",
                path.to_str().expect(PATH_TO_STRING_MSG)
            )
        })?;
        let mut json: serde_json::Value = serde_json::from_reader(file).with_context(|| {
            format!(
                "unable to read metadata as JSON from {}",
                path.to_str().expect(PATH_TO_STRING_MSG)
            )
        })?;

        // Update url fields
        let mut updated = update(&mut json, "image", base_uri)?;
        updated |= update(&mut json, "animation_url", base_uri)?;

        if updated {
            let mut file = fs::File::create(&path)?;
            write!(file, "{}", serde_json::to_string_pretty(&json)?)?;
            trace!("updated '{}'", path.to_str().expect(PATH_TO_STRING_MSG));
        } else {
            trace!(
                "no changes made to '{}'...",
                path.to_str().expect(PATH_TO_STRING_MSG)
            );
        }
    }

    Ok(())
}

fn update(json: &mut Value, field: &str, base_uri: &Url) -> Result<bool> {
    if let Some(url) = json.get(field).and_then(|v| v.as_str()) {
        let mut url = parse_url(url)?;
        let file = url.path_segments().unwrap().last().unwrap();
        url = base_uri.join(file)?;
        json[field] = Value::String(url.to_string());
        trace!("updated url of '{field}' to '{url}'");
        return Ok(true);
    }

    Ok(false)
}

fn parse_url(value: &str) -> Result<Url> {
    let url = Url::parse(value);
    if let Err(e) = url {
        return match e {
            ParseError::RelativeUrlWithoutBase => {
                // Create dummy absolute url from relative
                Ok(Url::from_str("https://localhost")?.join(value)?)
            }
            _ => Err(Error::new(e)),
        };
    }
    Ok(url.unwrap())
}
