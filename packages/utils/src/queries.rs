use cosmwasm_std::QuerierWrapper;
use wyndex::{asset::Asset, pair::SimulationResponse};

use crate::errors::OutpostError;

/// Queries the Wyndex pool for the amount of `to_denom` that can be received for `from_token`
/// IMPORTANT: you must provide the pair contract address for the simulation
pub fn query_wynd_pool_swap(
    querier: &QuerierWrapper,
    pool_address: String,
    from_token: &Asset,
    // just for error reporting purposes
    to_denom: String,
) -> Result<SimulationResponse, OutpostError> {
    wyndex::querier::simulate(querier, pool_address, from_token).map_err(|_| {
        OutpostError::SwapSimulationError {
            from: from_token.info.to_string(),
            to: to_denom,
        }
    })
}
