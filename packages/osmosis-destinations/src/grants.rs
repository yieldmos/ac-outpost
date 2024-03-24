use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CsdkCoin;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_grant_spec::grants::{ContractExecutionAuthorizationLimit, GrantBase, GrantRequirement};
use membrane::cdp;

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

pub fn membrane_deposit_into_stability_ool_grant(
    base: GrantBase,
    contract_addr: Addr,
    cdt_denom: &str,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::contract_exec_messages_auth(
        base,
        contract_addr,
        vec![&membrane::stability_pool::ExecuteMsg::Deposit { user: None }],
        Some(cdt_denom),
    )]
}

pub fn membrane_deposit_grant(
    base: GrantBase,
    contract_addr: Addr,
    position_id: Uint128,
    deposits: &Vec<Coin>,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::contract_exec_messages_auth(
        base,
        contract_addr,
        vec![&cdp::ExecuteMsg::Deposit {
            position_id: Some(position_id),
            position_owner: None,
        }],
        // We need to accept any basket denom, unsure if this will allow anythign or nothing
        None,
    )]
}

pub fn membrane_mint_cdt_grant(
    base: GrantBase,
    contract_addr: Addr,
    position_id: Uint128,
    desired_ltv: Decimal,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::contract_exec_messages_auth(
        base,
        contract_addr,
        vec![&membrane::cdp::ExecuteMsg::IncreaseDebt {
            position_id,
            amount: None,
            LTV: Some(desired_ltv),
            mint_to_addr: None,
        }],
        None,
    )]
}

pub fn membrane_repay_cdt_grant(
    base: GrantBase,
    contract_addr: Addr,
    position_id: Uint128,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::contract_exec_messages_auth(
        base,
        contract_addr,
        vec![&membrane::cdp::ExecuteMsg::Repay {
            position_id,
            position_owner: None,
            send_excess_to: None,
        }],
        // this should allow any basket denom, unsure if this will allow anythign or nothing
        None,
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
