use cosmwasm_std::{Addr};
use cw_grant_spec::grants::{
    GrantBase, GrantRequirement,
};
use white_whale::pool_network::asset::AssetInfo;

pub fn eris_lsd_grant(base: GrantBase, lsd_addr: Addr, asset: AssetInfo) -> Vec<GrantRequirement> {
    vec![match asset {
        AssetInfo::NativeToken { denom } => GrantRequirement::default_contract_exec_auth(
            base,
            lsd_addr,
            vec!["bond"],
            Some(denom.as_str()),
        ),
        AssetInfo::Token { contract_addr } => GrantRequirement::default_contract_exec_auth(
            base,
            Addr::unchecked(contract_addr),
            vec!["send"],
            None,
        ),
    }]
}
