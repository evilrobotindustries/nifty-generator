use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Serialize)]
pub struct Metadata<'a> {
    pub id: usize,
    // Name of the item.
    pub name: String,
    // A human readable description of the item. Markdown is supported.
    pub description: &'a str,
    /// This is the URL to the image of the item. Can be just about any type of image (including SVGs, which will be cached into PNGs by OpenSea), and can be IPFS URLs or paths. We recommend using a 350 x 350 image.
    pub image: String,
    // This is the URL that will appear below the asset's image on OpenSea and will allow users to leave OpenSea and view the item on your site.
    pub external_url: Option<String>,
    // These are the attributes for the item, which will show up on the OpenSea page for the item. (see below)
    pub attributes: Vec<Attribute<'a>>,
    // Background color of the item on OpenSea. Must be a six-character hexadecimal without a pre-pended #.
    pub background_color: Option<&'a str>,
    // A URL to a multi-media attachment for the item. The file extensions GLTF, GLB, WEBM, MP4, M4V, OGV, and OGG are supported, along with the audio-only extensions MP3, WAV, and OGA.
    // Animation_url also supports HTML pages, allowing you to build rich experiences and interactive NFTs using JavaScript canvas, WebGL, and more. Scripts and relative paths within the HTML page are now supported. However, access to browser extensions is not supported.
    pub animation_url: Option<String>,
    // A URL to a YouTube video.
    pub youtube_url: Option<String>,
}

pub enum Attribute<'a> {
    String {
        trait_type: &'a str,
        value: &'a str,
    },
    // Numeric
    Number {
        trait_type: &'a str,
        value: usize,
        max_value: Option<usize>,
    },
    BoostPercentage {
        trait_type: &'a str,
        value: f32,
        max_value: Option<usize>,
    },
    BoostNumber {
        trait_type: &'a str,
        value: f32,
        max_value: Option<usize>,
    },
    // Date
    Date {
        trait_type: &'a str,
        // A unix timestamp (seconds)
        value: i64,
    },
    // An attribute without any specific type
    Value(&'static str, String),
}

impl Serialize for Attribute<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const ATTRIBUTE_NAME: &str = "Attribute";
        match self {
            Attribute::String { trait_type, value } => {
                let mut state = serializer.serialize_struct(ATTRIBUTE_NAME, 2)?;
                state.serialize_field("trait_type", trait_type)?;
                state.serialize_field("value", value)?;
                state.end()
            }
            Attribute::Number {
                trait_type,
                value,
                max_value,
            } => {
                let mut state = serializer.serialize_struct(ATTRIBUTE_NAME, 4)?;
                state.serialize_field("display_type", "number")?;
                state.serialize_field("trait_type", trait_type)?;
                state.serialize_field("value", value)?;
                if let Some(max_value) = max_value {
                    state.serialize_field("max_value", max_value)?;
                }
                state.end()
            }
            Attribute::BoostPercentage {
                trait_type,
                value,
                max_value,
            } => {
                let mut state = serializer.serialize_struct(ATTRIBUTE_NAME, 4)?;
                state.serialize_field("display_type", "boost_percentage")?;
                state.serialize_field("trait_type", trait_type)?;
                state.serialize_field("value", value)?;
                if let Some(max_value) = max_value {
                    state.serialize_field("max_value", max_value)?;
                }
                state.end()
            }
            Attribute::BoostNumber {
                trait_type,
                value,
                max_value,
            } => {
                let mut state = serializer.serialize_struct(ATTRIBUTE_NAME, 4)?;
                state.serialize_field("display_type", "boost_number")?;
                state.serialize_field("trait_type", trait_type)?;
                state.serialize_field("value", value)?;
                if let Some(max_value) = max_value {
                    state.serialize_field("max_value", max_value)?;
                }
                state.end()
            }
            Attribute::Date { trait_type, value } => {
                let mut state = serializer.serialize_struct(ATTRIBUTE_NAME, 3)?;
                state.serialize_field("display_type", "date")?;
                state.serialize_field("trait_type", trait_type)?;
                state.serialize_field("value", value)?;
                state.end()
            }
            Attribute::Value(property, value) => {
                let mut state = serializer.serialize_struct(ATTRIBUTE_NAME, 1)?;
                state.serialize_field(property, value)?;
                state.end()
            }
        }
    }
}
