use cosmwasm_std::QuerierWrapper;
use wyndex::{
    asset::{Asset, AssetInfoValidated, AssetValidated},
    pair::SimulationResponse,
};

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

/// Queries the Wyndex multihop factory for the amount of `to_denom`
/// that can be received for a bunch of different tokens. This can be used
/// to compare the value of all the input offer tokens.
pub fn simulate_multiple_swaps(
    querier: &QuerierWrapper,
    offer_tokens: Vec<AssetValidated>,
    target_token: &AssetInfoValidated,
    multihop_factory_addr: &String,
) -> Result<Vec<(AssetValidated, SimulationResponse)>, OutpostError> {
    offer_tokens
        .into_iter()
        .map(|offer_token| {
            let simulation: SimulationResponse = querier
                .query_wasm_smart(
                    multihop_factory_addr,
                    &wyndex_multi_hop::msg::QueryMsg::SimulateSwapOperations {
                        offer_amount: offer_token.amount,
                        operations: vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
                            offer_asset_info: offer_token.info.clone().into(),
                            ask_asset_info: target_token.clone().into(),
                        }],
                        referral: false,
                        referral_commission: None,
                    },
                )
                .map_err(|_| OutpostError::SwapSimulationError {
                    from: offer_token.info.to_string(),
                    to: target_token.to_string(),
                })?;

            Ok((offer_token.clone(), simulation))
        })
        .collect::<Result<Vec<_>, _>>()
}
