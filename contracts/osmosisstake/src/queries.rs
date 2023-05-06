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

pub fn query_denom_market(querier: &QuerierWrapper, denom: String) -> StdResult<Market> {
    querier.query_wasm_smart(
        RED_BANK_ADDRESS,
        &mars_red_bank_types::red_bank::QueryMsg::Market { denom },
    )
}

/// Check that the given asset is depositable and how much
/// room is left before hitting the cap
pub fn depositable_token_amount(
    Market {
        collateral_total_scaled,
        deposit_cap,
        deposit_enabled,
        ..
    }: &Market,
) -> Result<Uint128, ContractError> {
    if *deposit_enabled {
        Ok(deposit_cap - collateral_total_scaled)
    } else {
        Ok(Uint128::zero())
    }
}

/// Check that depositing pays more than borrowing. If this is false then
/// levering up into the same asset likely isn't sensible.
pub fn deposit_rate_gt_borrow_rate(market: &Market) -> bool {
    market.borrow_rate;

    unimplemented!("need to determine the deposit reward rate");
}
