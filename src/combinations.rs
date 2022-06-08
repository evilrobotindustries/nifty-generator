use crate::config::{AttributeOption, Config};
use indexmap::IndexMap;
use log::{debug, trace};

pub(crate) type AttributeName = str;
pub(crate) type AttributeValue = str;

pub(crate) fn combinations(
    config: &Config,
) -> Vec<Vec<(&AttributeName, &AttributeValue, &Option<AttributeOption>)>> {
    let attributes = config.attributes.len();
    let total_options: usize = config.attributes.values().map(|a| a.len()).sum();
    trace!("generating combinations from {attributes} available attributes and a total of {total_options} attribute options");
    let combinations = generate(&config.attributes, 0);
    debug!(
        "generated {} combinations from {attributes} available attributes and a total of {total_options} attribute options",
        combinations.len(),
    );
    combinations
}

fn generate(
    attributes: &IndexMap<String, IndexMap<String, Option<AttributeOption>>>,
    i: usize,
) -> Vec<Vec<(&AttributeName, &AttributeValue, &Option<AttributeOption>)>> {
    let mut results = Vec::<Vec<(&str, &str, &Option<AttributeOption>)>>::new();
    let attribute = &attributes
        .get_index(i)
        .expect(&format!("could not get attribute at index {i}"));
    for (name, option) in attribute.1 {
        if i < attributes.len() - 1 {
            for mut x in generate(attributes, i + 1) {
                x.insert(0, (&attribute.0, &name, option));
                results.push(x);
            }
        } else {
            results.push(vec![(&attribute.0, &name, option)]);
        }
    }
    results
}
