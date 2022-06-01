use crate::config::Attribute;
use log::debug;
use rand::seq::SliceRandom;
use std::cmp;
use std::path::PathBuf;
use thousands::Separable;

pub(crate) fn random<'a>(
    items: Vec<Vec<(&'a Attribute, &'a str, &'a Option<PathBuf>)>>,
    amount: u32,
) -> Vec<Vec<(&'a Attribute, &'a str, &'a Option<PathBuf>)>> {
    let amount = cmp::min(amount as usize, items.len());
    debug!(
        "randomly selecting {} from {} items",
        amount.separate_with_commas(),
        items.len().separate_with_commas()
    );
    let mut rng = &mut rand::thread_rng();
    items.choose_multiple(&mut rng, amount).cloned().collect()
}
