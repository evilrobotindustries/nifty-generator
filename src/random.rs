use crate::config::{Attribute, MediaType};
use log::debug;
use rand::seq::SliceRandom;
use std::cmp;
use thousands::Separable;

pub(crate) fn random<'a>(
    items: Vec<Vec<(&'a Attribute, &'a str, &'a Option<MediaType>)>>,
    amount: u32,
) -> Vec<Vec<(&'a Attribute, &'a str, &'a Option<MediaType>)>> {
    let amount = cmp::min(amount as usize, items.len());
    debug!(
        "randomly selecting {} from {} items",
        amount.separate_with_commas(),
        items.len().separate_with_commas()
    );
    let mut rng = &mut rand::thread_rng();
    items.choose_multiple(&mut rng, amount).cloned().collect()
}
