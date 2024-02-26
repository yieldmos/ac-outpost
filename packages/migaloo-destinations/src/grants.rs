
use cosmwasm_std::{Addr};
use cw_grant_spec::grants::{
    GrantBase, GrantRequirement,
};
use sail_destinations::grants::eris_lsd_grant;
use white_whale::pool_network::asset::{AssetInfo};

use crate::comp_prefs::{AshAction, MigalooProjectAddrs};

pub fn furnace_grant(
    base: GrantBase,
    projects_addrs: MigalooProjectAddrs,
    and_then: Option<AshAction>,
    ash_asset: AssetInfo,
) -> Vec<GrantRequirement> {
    let grants = vec![GrantRequirement::default_contract_exec_auth(
        base.clone(),
        projects_addrs.furnace,
        vec!["burn"],
        Some("uwhale"),
    )];

    let then_grants = match and_then {
        Some(AshAction::AmpAsh) => eris_lsd_grant(base, projects_addrs.vaults.amp_ash, ash_asset),
        Some(AshAction::EcosystemStake) => {
            ecosystem_stake_grant(base, projects_addrs.ecosystem_stake, ash_asset)
        }
        None => vec![],
    };

    [grants, then_grants].concat()
}

pub fn ecosystem_stake_grant(
    base: GrantBase,
    stake_contract_addr: Addr,
    asset: AssetInfo,
) -> Vec<GrantRequirement> {
    vec![match asset {
        AssetInfo::NativeToken { denom } => GrantRequirement::default_contract_exec_auth(
            base,
            stake_contract_addr,
            vec!["stake"],
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
