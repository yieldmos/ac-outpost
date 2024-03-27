use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use serde::{de::DeserializeOwned, Serialize};

use crate::errors::OutpostError;

#[cw_serde]
pub struct CompoundPrefs<DestProjects> {
    pub relative: Vec<DestinationAction<DestProjects>>,
}

#[cw_serde]
pub struct DestinationAction<DestProjects> {
    pub destination: DestProjects,
    /// the percentage of the rewards that should be sent to this destination
    /// this is a number with 18 decimal places
    /// for example "250000000000000000" is 25%
    pub amount: u128,
}

#[cw_serde]
pub struct ValidatorSelection {
    pub validator_address: String,
    pub percent: Decimal,
}

#[cw_serde]
pub struct TakeRate {
    pub max_tax_fee: Decimal,
    pub take_rate_addr: Addr,
}

impl TakeRate {
    pub fn new(
        api: &dyn Api,
        max_tax_fee: Decimal,
        take_rate_address: &str,
    ) -> Result<Self, OutpostError> {
        Ok(TakeRate {
            max_tax_fee,
            take_rate_addr: api.addr_validate(take_rate_address)?,
        })
    }
}

/// Helper for storing submsg data
pub fn store_submsg_data<T>(
    store: &mut dyn Storage,
    data: T,
    latest_reply_id_state: Item<u64>,
    submsg_data: Map<&u64, T>,
) -> StdResult<u64>
where
    T: Serialize + DeserializeOwned,
{
    let next_reply_id = latest_reply_id_state.may_load(store)?.unwrap_or(0) + 1;
    latest_reply_id_state.save(store, &next_reply_id)?;
    submsg_data.save(store, &next_reply_id, &data)?;

    Ok(next_reply_id)
}
