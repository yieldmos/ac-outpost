use cosmwasm_std::{Addr, FullDelegation, QuerierWrapper};

use crate::{
    contract::{AllPendingRewards, PendingReward},
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
            |delegation| match querier.query_delegation(delegator, &delegation.validator) {
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
        &rewards.iter().map(|x| x.amount.clone()).collect(),
    );

    Ok(AllPendingRewards { rewards, total })
}
