use crate::config::Config;
use anyhow::{Context, Result};
use std::path::PathBuf;
use structopt::StructOpt;

mod combinations;
mod config;
mod generation;
mod metadata;
mod output;
mod random;

const PATH_TO_STRING_MSG: &str = "could not convert path to string";

fn main() -> Result<()> {
    // Initialise command line arguments and logging
    let args: Arguments = Arguments::from_args();
    loggerv::init_with_verbosity(args.verbosity).with_context(|| {
        format!(
            "unable to initialise logger with verbosity {}",
            args.verbosity
        )
    })?;

    // Read config from config.json
    let config = config::load(&args)?;

    // Initialise output directories
    output::init(&args)?;

    // Generate tokens
    generation::generate(args, &config)
}

/// Do fancy things
#[derive(StructOpt, Debug)]
#[structopt(
    name = "Nifty Generator",
    author = "Evil Robot Industries",
    about = "A NFT generation tool."
)]
struct Arguments {
    /// The configuration file name.
    #[structopt(long = "config", short = "c", default_value = "config.json")]
    config: String,

    /// The source directory, containing the required config.json configuration file.
    #[structopt(parse(from_os_str))]
    source: PathBuf,

    /// The logging verbosity: use multiple `v`s to increase verbosity.
    #[structopt(short = "v", long = "verbose", default_value = "1")]
    verbosity: u64,

    /// The output directory name.
    #[structopt(long = "output", short = "o", default_value = "output")]
    output: String,

    /// The output directory name for the resulting token media.
    #[structopt(long = "media", default_value = "media")]
    media: String,

    /// The output directory name for the resulting token metadata.
    #[structopt(long = "metadata", default_value = "metadata")]
    metadata: String,
}
