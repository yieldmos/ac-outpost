use cosmwasm_std::{Addr, FullDelegation, QuerierWrapper, Uint128};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};

use crate::{
    contract::{AllPendingRewards, PendingReward},
    execute::JUNO_NETA_PAIR_ADDR,
    helpers::sum_coins,
    msg::VersionResponse,
    ContractError,
};

pub fn query_version() -> VersionResponse {
    VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn query_pending_rewards(
    querier: &QuerierWrapper,
    delegator: &Addr,
) -> Result<AllPendingRewards, ContractError> {
    let staking_denom = querier.query_bonded_denom()?;

    let rewards_query: Result<Vec<PendingReward>, ContractError> = querier
        .query_all_delegations(delegator)?
        .into_iter()
        .map(
            |delegation| match querier.query_delegation(delegator, delegation.validator) {
                Ok(Some(FullDelegation {
                    validator,
                    accumulated_rewards,
                    ..
                })) => Ok(PendingReward {
                    validator,
                    amount: sum_coins(&staking_denom, &accumulated_rewards),
                }),
                _ => Err(ContractError::QueryPendingRewardsFailure),
            },
        )
        .collect();

    let rewards = rewards_query?;

    let total = sum_coins(
        &staking_denom,
        &rewards
            .iter()
            .map(|x| x.amount.clone())
            .collect::<Vec<cosmwasm_std::Coin>>()[..],
    );

    Ok(AllPendingRewards { rewards, total })
}

/// Queries the Wyndex pool for the amount of `to_denom` that can be received for `from_token`
/// IMPORTANT: you must provide the pair contract address for the simulation
pub fn query_wynd_pool_swap(
    querier: &QuerierWrapper,
    pool_address: String,
    from_token: &Asset,
    // just for error reporting purposes
    to_denom: &String,
) -> Result<SimulationResponse, ContractError> {
    wyndex::querier::simulate(querier, pool_address, from_token).map_err(|_| {
        ContractError::SwapSimulationError {
            from: from_token.info.to_string(),
            to: to_denom.to_string(),
        }
    })
}

/// Queries wyndex for the amount of neta that can be received for `from_token_amount` of juno
pub fn query_juno_neta_swap(
    querier: &QuerierWrapper,
    from_token_amount: Uint128,
) -> Result<SimulationResponse, ContractError> {
    query_wynd_pool_swap(
        querier,
        JUNO_NETA_PAIR_ADDR.to_string(),
        &Asset {
            info: AssetInfo::Native("ujuno".to_string()),
            amount: from_token_amount,
        },
        &"uneta".to_string(),
    )
}

/// Queries wyndex for the amount of wynd that can be received for `from_token_amount` of juno
pub fn query_juno_wynd_swap(
    querier: &QuerierWrapper,
    from_token_amount: Uint128,
) -> Result<SimulationResponse, ContractError> {
    query_wynd_pool_swap(
        querier,
        JUNO_NETA_PAIR_ADDR.to_string(),
        &Asset {
            info: AssetInfo::Native("ujuno".to_string()),
            amount: from_token_amount,
        },
        &"uwynd".to_string(),
    )
}
