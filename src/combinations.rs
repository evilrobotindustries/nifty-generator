use crate::config::{Attribute, Config, MediaType};
use log::{debug, trace};

pub(crate) fn combinations(config: &Config) -> Vec<Vec<(&Attribute, &str, &Option<MediaType>)>> {
    let attributes = config.attributes.len();
    let total_options: usize = config.attributes.iter().map(|a| a.options.len()).sum();
    trace!("generating combinations from {attributes} available attributes and a total of {total_options} attribute options");
    let combinations = generate(&config.attributes, 0);
    debug!(
        "generated {} combinations from {attributes} available attributes and a total of {total_options} attribute options",
        combinations.len(),
    );
    combinations
}

fn generate(
    attributes: &Vec<Attribute>,
    i: usize,
) -> Vec<Vec<(&Attribute, &str, &Option<MediaType>)>> {
    let mut results = Vec::<Vec<(&Attribute, &str, &Option<MediaType>)>>::new();
    let attribute = &attributes[i];
    for (name, path) in &attribute.options {
        if i < attributes.len() - 1 {
            for mut x in generate(attributes, i + 1) {
                x.insert(0, (&attribute, &name, path));
                results.push(x);
            }
        } else {
            results.push(vec![(&attribute, &name, path)]);
        }
    }
    results
}
