use cosmwasm_std::{Order, Storage, Uint64};

use crate::state::ALLOWED_STRATEGY_IDS;

/// Queries the list of all strategy ids from state
pub fn all_strat_ids(store: &dyn Storage) -> Vec<Uint64> {
    ALLOWED_STRATEGY_IDS
        .keys(store, None, None, Order::Ascending)
        .filter_map(Result::ok)
        .map(Uint64::from)
        .collect()
}
