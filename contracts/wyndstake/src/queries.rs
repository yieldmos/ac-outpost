use cosmwasm_std::{Addr, Deps, QuerierWrapper, Uint128};
use wynd_helpers::wynd_swap::simulate_wynd_pool_swap;
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};
use wyndex_multi_hop::msg::SwapOperation;

use crate::{
    execute::{
        JUNO_WYND_PAIR_ADDR, NETA_CW20_ADDR, WYND_CW20_ADDR, WYND_CW20_STAKING_ADDR,
        WYND_MULTI_HOP_ADDR,
    },
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

pub fn query_pending_wynd_rewards(
    querier: &QuerierWrapper,
    delegator: &Addr,
) -> Result<Uint128, ContractError> {
    let rewards: wynd_stake::msg::RewardsResponse = querier.query_wasm_smart(
        WYND_CW20_STAKING_ADDR,
        &wynd_stake::msg::QueryMsg::WithdrawableRewards {
            owner: delegator.to_string(),
        },
    )?;

    Ok(rewards.rewards)
}

/// Queries wyndex for the amount of juno that can be received for `from_token_amount` of wynd
pub fn query_wynd_juno_swap(
    querier: &QuerierWrapper,
    from_token_amount: Uint128,
) -> Result<SimulationResponse, ContractError> {
    simulate_wynd_pool_swap(
        querier,
        JUNO_WYND_PAIR_ADDR.to_string(),
        &Asset {
            info: AssetInfo::Token(WYND_CW20_ADDR.to_string()),
            amount: from_token_amount,
        },
        "ujuno".to_string(),
    )
    .map_err(|e| ContractError::from(e))
}

/// Queries wyndex for the amount of neta that can be received for `from_token_amount` of wynd
pub fn query_wynd_neta_swap(
    querier: &QuerierWrapper,
    from_token_amount: Uint128,
) -> Result<(SimulationResponse, Vec<SwapOperation>), ContractError> {
    let operations = vec![
        SwapOperation::WyndexSwap {
            offer_asset_info: AssetInfo::Token(WYND_CW20_ADDR.to_string()),
            ask_asset_info: AssetInfo::Native("ujuno".to_string()),
        },
        SwapOperation::WyndexSwap {
            offer_asset_info: AssetInfo::Native("ujuno".to_string()),
            ask_asset_info: AssetInfo::Token(NETA_CW20_ADDR.to_string()),
        },
    ];

    let sim_resp = querier.query_wasm_smart(
        WYND_MULTI_HOP_ADDR.to_string(),
        &wyndex_multi_hop::msg::QueryMsg::SimulateSwapOperations {
            offer_amount: from_token_amount,
            operations: operations.clone(),
            referral: false,
            referral_commission: None,
        },
    )?;

    Ok((sim_resp, operations))
}
