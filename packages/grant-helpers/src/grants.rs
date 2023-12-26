use cosmwasm_std::{Addr, Coin};
use cw_grant_spec::grants::{
    AuthorizationType, GrantBase, GrantRequirement, StakeAuthorizationPolicy,
    StakeAuthorizationType, StakeAuthorizationValidators,
};

pub fn native_staking_grant(
    GrantBase {
        granter,
        grantee,
        expiration,
    }: GrantBase,
    max_tokens: Option<Coin>,
    validators: Option<Vec<String>>,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::GrantSpec {
        grant_type: AuthorizationType::StakeAuthorization {
            max_tokens,
            authorization_type: StakeAuthorizationType::Delegate,
            validators: validators.map(|vs| {
                StakeAuthorizationPolicy::AllowList(StakeAuthorizationValidators { address: vs })
            }),
        },
        granter,
        grantee,
        expiration,
    }]
}

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
