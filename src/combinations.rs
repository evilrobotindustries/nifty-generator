use crate::config::{Attribute, Config};
use std::path::PathBuf;

pub(crate) fn combinations(config: &Config) -> Vec<Vec<(&Attribute, &str, &Option<PathBuf>)>> {
    generate(&config.attributes, 0)
}

fn generate(
    attributes: &Vec<Attribute>,
    i: usize,
) -> Vec<Vec<(&Attribute, &str, &Option<PathBuf>)>> {
    let mut results = Vec::<Vec<(&Attribute, &str, &Option<PathBuf>)>>::new();
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
