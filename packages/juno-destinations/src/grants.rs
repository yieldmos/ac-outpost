use cosmwasm_std::{Addr};
use cw_grant_spec::grants::{
    GrantBase, GrantRequirement,
};

pub fn balance_dao_grant(base: GrantBase, contract_addr: Addr) -> Vec<GrantRequirement> {
    vec![GrantRequirement::default_contract_exec_auth(
        base,
        contract_addr,
        vec!["swap"],
        Some("ujuno"),
    )]
}

pub fn gelotto_lottery_grant(base: GrantBase, contract_addr: Addr) -> Vec<GrantRequirement> {
    vec![GrantRequirement::default_contract_exec_auth(
        base,
        contract_addr,
        vec!["sender_buy_seed"],
        Some("ujuno"),
    )]
}

pub fn wyndao_staking_grant(base: GrantBase, contract_addr: Addr) -> Vec<GrantRequirement> {
    vec![GrantRequirement::default_contract_exec_auth(
        base,
        contract_addr,
        vec!["delegate"],
        None,
    )]
}
