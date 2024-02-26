use cosmwasm_std::{coin, Addr, Coin};
use cw_grant_spec::grants::{
    AuthorizationType, GrantBase, GrantRequirement, StakeAuthorizationPolicy,
    StakeAuthorizationType, StakeAuthorizationValidators,
};
use white_whale::pool_network::asset::AssetInfo;

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

pub fn native_send_token(
    base: GrantBase,
    denom: AssetInfo,
    destination_wallet_address: String,
) -> Vec<GrantRequirement> {
    let GrantBase {
        granter,
        grantee,
        expiration,
    } = base.clone();

    vec![match denom {
        // if it's a native denom we need a send authorization
        AssetInfo::NativeToken { denom } => GrantRequirement::GrantSpec {
            grant_type: AuthorizationType::SendAuthorization {
                spend_limit: Some(vec![coin(u128::MAX, denom)]),
                allow_list: Some(vec![Addr::unchecked(destination_wallet_address)]),
            },
            granter: granter.clone(),
            grantee: grantee.clone(),
            expiration,
        },
        // if it's a cw20 then we need a contract execution authorization on the cw20 contract
        AssetInfo::Token { contract_addr } => GrantRequirement::default_contract_exec_auth(
            base,
            Addr::unchecked(contract_addr),
            vec!["transfer"],
            None,
        ),
    }]
}
