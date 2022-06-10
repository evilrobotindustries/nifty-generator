use crate::config::{Attribute, AttributeOption};
use crate::Config;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use log::debug;
use rand::distributions::{Distribution, WeightedIndex};
use thousands::Separable;

pub(crate) type AttributeValue = str;

pub(crate) fn generate(
    config: &Config,
) -> Result<Vec<Vec<(&Attribute, &AttributeValue, &AttributeOption)>>> {
    debug!(
        "randomly generating {} items of each attribute, using the weights specified in config...",
        config.supply.separate_with_commas(),
    );

    let mut rng = &mut rand::thread_rng();
    let mut results: IndexMap<&Attribute, Vec<(&AttributeValue, &AttributeOption)>> =
        IndexMap::with_capacity(config.attributes.len());
    let mut stats: IndexMap<&str, IndexMap<&str, Stats>> = IndexMap::new();
    for attribute in &config.attributes {
        let options = &attribute.options;
        let weighted_index = WeightedIndex::new(options.values().map(|option| option.weight()))
            .with_context(|| {
                format!(
                    "failed to generate the weighted index for the {} attribute",
                    attribute.name
                )
            })?;

        let generated: Vec<(&AttributeValue, &AttributeOption)> = (0..config.supply)
            .map(|_| {
                let i = weighted_index.sample(&mut rng);
                options
                    .get_index(i)
                    .map(|k| (k.0.as_ref(), k.1))
                    .expect(&format!("failed to get the attribute value at index {i}"))
            })
            .collect();

        let total_weight = attribute
            .options
            .values()
            .map(|option| option.weight())
            .sum();
        let attribute_stats = attribute
            .options
            .iter()
            .map(|(value, _)| {
                (
                    value.as_ref(),
                    Stats {
                        weight: *attribute.options[value].weight(),
                        total_weight,
                        count: 0,
                        total_items: generated.len(),
                    },
                )
            })
            .collect();
        stats.insert(
            &attribute.name,
            generated.iter().fold(attribute_stats, |mut f, value| {
                f[value.0].count += 1;
                f
            }),
        );

        results.insert(&attribute, generated);
    }
    let results = (0..config.supply).fold(Vec::with_capacity(config.supply), |mut v, i| {
        let attributes: Vec<(&Attribute, &AttributeValue, &AttributeOption)> = results
            .iter()
            .map(|(attribute, options)| (*attribute, options[i].0, options[i].1))
            .collect();
        v.push(attributes);
        v
    });

    debug!("generation complete, outputting attribute stats...");
    for (attribute, mut stats) in stats.into_iter().rev() {
        stats.sort_by(|k, _, k2, _| k.cmp(k2));

        debug!(
            "'{attribute}' = {}",
            stats
                .iter()
                .map(|v| format!(
                    "'{}': expected {:.2}% vs {:.2}% actual",
                    v.0,
                    v.1.expected_weight_percentage(),
                    v.1.actual_percentage()
                ))
                .collect::<Vec<String>>()
                .join(", ")
        );
    }
    // todo: include stats on duplicates

    Ok(results)
}

#[derive(Debug)]
struct Stats {
    weight: f64,
    total_weight: f64,
    count: usize,
    total_items: usize,
}

impl Stats {
    fn expected_weight_percentage(&self) -> f64 {
        (self.weight / self.total_weight) * 100.0
    }

    fn actual_percentage(&self) -> f64 {
        (self.count as f64 / self.total_items as f64) * 100.0
    }
}
