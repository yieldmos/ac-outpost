use cosmwasm_std::{Addr, Uint128};
use cw_grant_spec::grants::{
    GrantBase, GrantRequirement,
};

use crate::mars_types::RedBankExecuteMsgs;

pub fn membrane_stake_grant(
    base: GrantBase,
    contract_addr: Addr,
    mbrn_denom: &str,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::contract_exec_messages_auth(
        base,
        contract_addr,
        vec![&membrane::staking::ExecuteMsg::Stake { user: None }],
        Some(mbrn_denom),
    )]
}

pub fn membrane_deposit_grant(
    base: GrantBase,
    contract_addr: Addr,
    position_id: Uint128,
    _asset: String,
    mbrn_denom: &str,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::contract_exec_messages_auth(
        base,
        contract_addr,
        vec![&membrane::cdp::ExecuteMsg::Deposit {
            position_id: Some(position_id),
            position_owner: None,
        }],
        Some(mbrn_denom),
    )]
}

pub fn mint_milk_tia_grant(
    base: GrantBase,
    contract_addr: Addr,
    tia_denom: &str,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::default_contract_exec_auth(
        base,
        contract_addr,
        vec!["liquid_stake"],
        Some(tia_denom),
    )]
}

pub fn red_bank_repay_grant(
    base: GrantBase,
    contract_addr: Addr,
    account_id: String,
    repay_denom: &str,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::contract_exec_messages_auth(
        base,
        contract_addr,
        vec![&RedBankExecuteMsgs::RepayFromWallet { account_id }],
        Some(repay_denom),
    )]
}

pub fn stake_ion_grants(
    base: GrantBase,
    contract_addr: Addr,
    ion_denom: &str,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::default_contract_exec_auth(
        base,
        contract_addr,
        vec!["stake"],
        Some(ion_denom),
    )]
}
