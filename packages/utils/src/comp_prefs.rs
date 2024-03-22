use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal};

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
