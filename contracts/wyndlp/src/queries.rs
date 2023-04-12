use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};
use outpost_utils::comp_prefs::WyndLPBondingPeriod;
use wyndex::{asset::AssetValidated, pair::PairInfo};
use wyndex_stake::msg::{AllStakedResponse, WithdrawableRewardsResponse};

use crate::{execute::WYNDDEX_FACTORY_ADDR, msg::VersionResponse, ContractError};

pub fn query_version() -> VersionResponse {
    VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Queries the current user's pools that have rewards from the list of every pool in wyndex
pub fn query_current_user_pools(
    querier: &QuerierWrapper,
    delegator_addr: &Addr,
) -> StdResult<Vec<(PairInfo, Vec<AssetValidated>)>> {
    let pools: wyndex::factory::PairsResponse = querier.query_wasm_smart(
        WYNDDEX_FACTORY_ADDR.to_string(),
        &wyndex::factory::QueryMsg::Pairs {
            start_after: None,
            limit: None,
        },
    )?;

    check_user_pools_for_rewards(querier, delegator_addr, pools.pairs)
}

/// Given a list of user pools, check each to see if the given user has rewards
pub fn check_user_pools_for_rewards(
    querier: &QuerierWrapper,
    delegator_addr: &Addr,
    user_pools: Vec<PairInfo>,
) -> StdResult<Vec<(PairInfo, Vec<AssetValidated>)>> {
    Ok(user_pools
        .iter()
        .filter_map(|pair| {
            query_pending_rewards(querier, &pair.staking_addr, delegator_addr)
                .map(|rewards| (pair.clone(), rewards))
        })
        .collect::<Vec<(PairInfo, Vec<AssetValidated>)>>())
}

/// Queries the current user's rewards from from a specific pool's staking address.
/// If no rewards are found, returns None
pub fn query_pending_rewards(
    querier: &QuerierWrapper,
    pool_addr: &Addr,
    delegator: &Addr,
) -> Option<Vec<AssetValidated>> {
    let rewards_resp: StdResult<WithdrawableRewardsResponse> = querier.query_wasm_smart(
        pool_addr.to_string(),
        &wyndex_stake::msg::QueryMsg::WithdrawableRewards {
            owner: delegator.to_string(),
        },
    );

    if let Ok(WithdrawableRewardsResponse { rewards }) = rewards_resp {
        let pending_rewards: Vec<AssetValidated> = rewards
            .into_iter()
            .filter(|asset| asset.amount > Uint128::zero())
            .collect();

        if pending_rewards.len() > 0 {
            return Some(pending_rewards);
        }
    }

    None
}

/// Queries the current user's staked amount from a specific pool's staking address.
/// Returns the highest bonding period found
pub fn get_max_user_pool_bonding_period(
    querier: &QuerierWrapper,
    pool_addr: &Addr,
    delegator_addr: &Addr,
) -> Result<WyndLPBondingPeriod, ContractError> {
    let AllStakedResponse { stakes }: AllStakedResponse = querier.query_wasm_smart(
        pool_addr.to_string(),
        &wyndex_stake::msg::QueryMsg::AllStaked {
            address: delegator_addr.to_string(),
        },
    )?;

    let max_bonding_period: WyndLPBondingPeriod = stakes
        .iter()
        .map(|stake| stake.unbonding_period)
        .max()
        .ok_or_else(|| ContractError::NoPoolUnbondingPeriod {
            user: delegator_addr.to_string(),
            pool: pool_addr.to_string(),
        })?
        .try_into()?;

    Ok(max_bonding_period)
}
