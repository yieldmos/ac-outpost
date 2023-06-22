use cosmwasm_std::{Addr, Coin, FullDelegation, QuerierWrapper};

use crate::{errors::OutpostError, helpers::sum_coins};

pub struct AllPendingRewards {
    pub rewards: Vec<PendingReward>,
    pub total: Coin,
}

pub struct PendingReward {
    pub validator: String,
    pub amount: Coin,
}

/// Queries the pending staking rewards for a given delegator
pub fn query_pending_rewards(
    querier: &QuerierWrapper,
    delegator: &Addr,
    staking_denom: String,
) -> Result<AllPendingRewards, OutpostError> {
    // gets all of the individual delegations for the delegator
    let rewards_query: Result<Vec<PendingReward>, OutpostError> = querier
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
                _ => Err(OutpostError::QueryPendingRewardsFailure),
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
