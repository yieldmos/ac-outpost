use crate::{
    execute::RED_BANK_ADDRESS,
    msg::{AuthorizedCompoundersResponse, VersionResponse},
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};
use cosmwasm_std::{Addr, Deps, QuerierWrapper, StdResult, Uint128};
use mars_red_bank_types::red_bank::Market;

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

/// Check that the given asset is depositable and how much room is left before hitting the cap
pub fn query_depositable_token_amount(
    querier: &QuerierWrapper,
    denom: String,
) -> Result<Uint128, ContractError> {
    let market: Market = querier.query_wasm_smart(
        RED_BANK_ADDRESS,
        &mars_red_bank_types::red_bank::QueryMsg::Market {
            denom: denom.clone(),
        },
    )?;

    if market.deposit_enabled {
        Ok(market.deposit_cap - market.collateral_total_scaled)
    } else {
        Err(ContractError::DepositDisabled(denom))
    }
}
