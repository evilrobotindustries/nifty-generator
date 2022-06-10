use crate::config::Config;
use anyhow::{Context, Result};
use log::{error, trace};
use std::path::PathBuf;
use structopt::StructOpt;
use url::Url;

mod config;
mod deployment;
mod generation;
mod metadata;
mod output;
mod random;

const PATH_TO_STRING_MSG: &str = "could not convert path to string";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise command line arguments and logging
    let command = Command::from_args();
    loggerv::init_with_verbosity(command.verbosity()).with_context(|| {
        format!(
            "unable to initialise logger with verbosity {}",
            command.verbosity()
        )
    })?;

    match &command {
        Command::Generate {
            config,
            output,
            media,
            metadata,
            source,
            ..
        } => {
            // Read config from config.json
            let config = config::load(source, config)?;
            // Initialise output directories
            output::init(source, output, media, metadata)?;
            // Generate tokens
            generation::generate(source, output, media, metadata, config).await
        }
        Command::Deploy {
            output,
            metadata,
            base_uri,
            source,
            ..
        } => {
            trace!("attempting to parse {base_uri} as a url...");
            if !base_uri.ends_with("/") {
                error!("base uri of '{base_uri}' does not end with a '/'");
                return Ok(());
            }

            let base_uri = Url::parse(base_uri)
                .with_context(|| format!("unable to parse {base_uri} as a url"))?;
            deployment::deploy(source, output, metadata, &base_uri)
        }
    }
}

#[derive(StructOpt)]
#[structopt(
    name = "Nifty Generator",
    author = "Evil Robot Industries",
    about = "A NFT generation tool."
)]
enum Command {
    /// Generates the nifty media and metadata based on configuration.
    Generate {
        /// The configuration file name.
        #[structopt(long = "config", short = "c", default_value = "config.json")]
        config: String,

        /// The output directory name.
        #[structopt(long = "output", short = "o", default_value = "output")]
        output: String,

        /// The output directory name for the resulting token media.
        #[structopt(long = "media", default_value = "media")]
        media: String,

        /// The output directory name for the resulting token metadata.
        #[structopt(long = "metadata", default_value = "metadata")]
        metadata: String,

        /// The logging verbosity: use multiple `v`s to increase verbosity.
        #[structopt(short = "v", long = "verbose", default_value = "1")]
        verbosity: u64,

        /// The source directory, containing the required config.json configuration file.
        #[structopt(parse(from_os_str))]
        source: PathBuf,
    },
    /// Updates the metadata to point to the deployed media.
    Deploy {
        /// The output directory name.
        #[structopt(long = "output", short = "o", default_value = "output")]
        output: String,

        /// The output directory name for the resulting token metadata.
        #[structopt(long = "metadata", default_value = "metadata")]
        metadata: String,

        /// The base uri of the deployed media.
        #[structopt(long = "base-uri")]
        base_uri: String,

        /// The logging verbosity: use multiple `v`s to increase verbosity.
        #[structopt(short = "v", long = "verbose", default_value = "1")]
        verbosity: u64,

        /// The source directory, containing the required config.json configuration file.
        #[structopt(parse(from_os_str))]
        source: PathBuf,
    },
}

impl Command {
    fn verbosity(&self) -> u64 {
        match self {
            Command::Generate { verbosity, .. } => *verbosity,
            Command::Deploy { verbosity, .. } => *verbosity,
        }
    }
}
