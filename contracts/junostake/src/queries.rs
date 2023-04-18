use cosmwasm_std::{Addr, Deps, FullDelegation, QuerierWrapper, Uint128};
use outpost_utils::helpers::sum_coins;
use wynd_helpers::wynd_swap::simulate_wynd_pool_swap;
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};

use crate::{
    contract::{AllPendingRewards, PendingReward},
    execute::JUNO_NETA_PAIR_ADDR,
    msg::{AuthorizedCompoundersResponse, VersionResponse},
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};

pub fn query_version() -> VersionResponse {
    VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn query_authorized_compounders(deps: Deps) -> AuthorizedCompoundersResponse {
    let authorized_compound_addresses: Vec<Addr> =
        AUTHORIZED_ADDRS.load(deps.storage).unwrap_or(vec![]);
    let admin: Addr = ADMIN.load(deps.storage).unwrap();
    AuthorizedCompoundersResponse {
        admin,
        authorized_compound_addresses,
    }
}

/// Queries the pending staking rewards for a given delegator
pub fn query_pending_rewards(
    querier: &QuerierWrapper,
    delegator: &Addr,
    staking_denom: String,
) -> Result<AllPendingRewards, ContractError> {
    // gets all of the individual delegations for the delegator
    let rewards_query: Result<Vec<PendingReward>, ContractError> = querier
        .query_all_delegations(delegator)?
        .into_iter()
        .map(
            // each delegation is queried for its pending rewards
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

    // sums the rewards
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
    simulate_wynd_pool_swap(
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
    simulate_wynd_pool_swap(
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
