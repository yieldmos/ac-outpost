use crate::msg::{CompPrefsWithAddresses, OsmostakeCompoundPrefs, QueryMsg};
use crate::{
    msg::{AuthorizedCompoundersResponse, VersionResponse},
    state::{ADMIN, AUTHORIZED_ADDRS},
};

use cosmwasm_std::{Addr, Decimal, Deps, StdResult, Timestamp};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{
    AuthorizationType, GrantBase, GrantRequirement, RevokeRequirement,
};
use osmosis_destinations::comp_prefs::{OsmosisDestinationProject, OsmosisLsd, OsmosisPoolSettings};
use osmosis_destinations::grants::{membrane_stake_grant, mint_milk_tia_grant, stake_ion_grants};
use osmosis_helpers::osmosis_lp::{join_cl_pool_grants, join_classic_pool_grants};
use osmosis_helpers::osmosis_swap::osmosis_swap_grants;
use sail_destinations::grants::eris_lsd_grant;
use universal_destinations::grants::{native_send_token, native_staking_grant};
use white_whale::pool_network::asset::AssetInfo;
use withdraw_rewards_tax_grant::msg::GrantSpecData;


pub fn query_version() -> VersionResponse {
    VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn query_authorized_compounders(deps: Deps) -> AuthorizedCompoundersResponse {
    let authorized_compound_addresses: Vec<Addr> = AUTHORIZED_ADDRS.load(deps.storage).unwrap_or_default();
    let admin: Addr = ADMIN.load(deps.storage).unwrap();
    AuthorizedCompoundersResponse {
        admin,
        authorized_compound_addresses,
    }
}

impl Grantable for QueryMsg {
    type GrantSettings = CompPrefsWithAddresses;

    fn query_grants(
        grant_structure: GrantStructure<Self::GrantSettings>,
        current_timestamp: Timestamp,
    ) -> StdResult<Vec<GrantRequirement>> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            grant_data:
                CompPrefsWithAddresses {
                    comp_prefs: OsmostakeCompoundPrefs { comp_prefs: _, tax_fee, .. },
                    project_addresses,
                },
            ..
        } = grant_structure.clone();

        let withdraw_tax_grants = withdraw_rewards_tax_grant::msg::QueryMsg::query_grants(
            GrantStructure {
                granter,
                grantee: outpost_contract,
                expiration,
                grant_contract: Addr::unchecked(project_addresses.authzpp.withdraw_tax),
                grant_data: GrantSpecData {
                    taxation_addr: Addr::unchecked(project_addresses.take_rate_addr),
                    max_fee_percentage: tax_fee.unwrap_or(Decimal::MAX),
                },
            },
            current_timestamp,
        )?;

        Ok([withdraw_tax_grants, gen_comp_pref_grants(grant_structure)?].concat())
    }

    fn query_revokes(
        grant_structure: GrantStructure<Self::GrantSettings>,
    ) -> StdResult<Vec<cw_grant_spec::grants::RevokeRequirement>> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            ..
        } = grant_structure.clone();
        let taxation_revoke: Vec<RevokeRequirement> = vec![GrantRequirement::GrantSpec {
            grant_type: AuthorizationType::SendAuthorization {
                spend_limit: None,
                allow_list: None,
            },
            granter,
            grantee: outpost_contract,
            expiration,
        }
        .into()];

        Ok([
            taxation_revoke,
            gen_comp_pref_grants(grant_structure)?
                .into_iter()
                .map(|grant| -> RevokeRequirement { grant.into() })
                .collect(),
        ]
        .concat())
    }
}

pub fn gen_comp_pref_grants(
    GrantStructure {
        granter,
        grantee,
        expiration,
        grant_contract: _grant_contract,
        grant_data:
            CompPrefsWithAddresses {
                comp_prefs: OsmostakeCompoundPrefs { comp_prefs, .. },
                project_addresses,
                ..
            },
    }: GrantStructure<CompPrefsWithAddresses>,
) -> StdResult<Vec<GrantRequirement>> {
    let grant_specs = comp_prefs.relative.iter().flat_map(|action| -> Vec<GrantRequirement> {
        let base = GrantBase {
            granter: granter.clone(),
            grantee: grantee.clone(),
            expiration,
        };

        match action.destination.clone() {
            OsmosisDestinationProject::Unallocated {} => vec![],
            OsmosisDestinationProject::OsmosisStaking { validator_address } => {
                native_staking_grant(base, None, Some(vec![validator_address]))
            }
            OsmosisDestinationProject::TokenSwap { target_asset: _ } => osmosis_swap_grants(base),
            OsmosisDestinationProject::SendTokens { address, target_asset } => vec![
                osmosis_swap_grants(base.clone()),
                native_send_token(
                    base,
                    AssetInfo::NativeToken {
                        denom: target_asset.denom,
                    },
                    address,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::MintLsd { lsd: OsmosisLsd::Eris } => eris_lsd_grant(
                base,
                project_addresses.destination_projects.projects.eris_amposmo_bonding.clone(),
                AssetInfo::NativeToken {
                    denom: "uosmo".to_string(),
                },
            ),
            OsmosisDestinationProject::MintLsd {
                lsd: OsmosisLsd::MilkyWay,
            } => vec![
                osmosis_swap_grants(base.clone()),
                mint_milk_tia_grant(
                    base,
                    project_addresses.destination_projects.projects.milky_way_bonding.clone(),
                    &project_addresses.destination_projects.denoms.tia,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::IonStaking {} => vec![
                osmosis_swap_grants(base.clone()),
                stake_ion_grants(
                    base,
                    project_addresses.destination_projects.projects.ion_dao.clone(),
                    &project_addresses.destination_projects.denoms.ion,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::MembraneStake {} => vec![
                osmosis_swap_grants(base.clone()),
                membrane_stake_grant(
                    base.clone(),
                    project_addresses.destination_projects.projects.membrane.staking.clone(),
                    &project_addresses.destination_projects.denoms.mbrn,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::OsmosisLiquidityPool {
                pool_id: _,
                pool_settings: OsmosisPoolSettings::Standard { bond_tokens },
            } => join_classic_pool_grants(base, bond_tokens),
            OsmosisDestinationProject::OsmosisLiquidityPool {
                pool_id: _,
                pool_settings: OsmosisPoolSettings::ConcentratedLiquidity { .. },
            } => join_cl_pool_grants(base),
        }
    });

    Ok(dedupe_grant_reqs(grant_specs.collect()))
}
