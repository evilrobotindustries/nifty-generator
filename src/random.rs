use crate::combinations::{AttributeName, AttributeValue};
use crate::config::AttributeOption;
use log::debug;
use rand::seq::SliceRandom;
use std::cmp;
use thousands::Separable;

pub(crate) fn random<'a>(
    items: Vec<
        Vec<(
            &'a AttributeName,
            &'a AttributeValue,
            &'a Option<AttributeOption>,
        )>,
    >,
    amount: u32,
) -> Vec<
    Vec<(
        &'a AttributeName,
        &'a AttributeValue,
        &'a Option<AttributeOption>,
    )>,
> {
    let amount = cmp::min(amount as usize, items.len());
    debug!(
        "randomly selecting {} from {} items",
        amount.separate_with_commas(),
        items.len().separate_with_commas()
    );
    let mut rng = &mut rand::thread_rng();
    items.choose_multiple(&mut rng, amount).cloned().collect()
}
