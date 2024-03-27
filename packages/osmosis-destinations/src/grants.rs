use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CsdkCoin;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_grant_spec::grants::{ContractExecutionAuthorizationLimit, GrantBase, GrantRequirement};

use crate::mars_types::RedBankExecuteMsgs;

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
