use cosmwasm_std::{Addr, FullDelegation, QuerierWrapper, Uint128};
use outpost_utils::{helpers::sum_coins, queries::query_wynd_pool_swap};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};

use crate::{
    contract::{AllPendingRewards, PendingReward},
    execute::JUNO_NETA_PAIR_ADDR,
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
    staking_denom: String,
) -> Result<AllPendingRewards, ContractError> {
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
        "uneta".to_string(),
    )
    .map_err(|e| ContractError::from(e))
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
        "uwynd".to_string(),
    )
    .map_err(|e| ContractError::from(e))
}
